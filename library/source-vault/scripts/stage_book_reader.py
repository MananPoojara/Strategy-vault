#!/usr/bin/env python3
"""Stage enumerated, completed original books for a read-only Docker reader.

No discovery walk, PDF transformation, overwrite of conflicting cached books,
or bibliographic/performance certification. Linux dirfd/O_NOFOLLOW operations
confine every source and cache path; streaming buffers are bounded to 1 MiB.
PDFs remain inline; DOC/DOCX/DJVU/EPUB originals are download attachments.
Run --self-test for private small fixtures before coordinating a full copy.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import sys
import tempfile
import unittest
from unittest import mock
import uuid

BUFFER = 1024 * 1024
NOFOLLOW = os.O_NOFOLLOW
DIR_FLAGS = os.O_RDONLY | os.O_DIRECTORY | NOFOLLOW
COMPLETED = {"copied", "reused_local_identical_sha256", "already_present_identical"}
DEFAULT_CACHE = Path("/home/server/.cache/strategy-vault-webapp/books")
DEFAULT_INDEX = Path("/home/server/.cache/strategy-vault-webapp/book-index.json")
FORMATS = {".pdf": ("pdf", "application/pdf", "inline"),
           ".doc": ("doc", "application/msword", "attachment"),
           ".docx": ("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "attachment"),
           ".djvu": ("djvu", "image/vnd.djvu", "attachment"),
           ".epub": ("epub", "application/epub+zip", "attachment")}


@dataclass(frozen=True)
class Record:
    relative: str
    size: int
    mtime_ns: int | None = None
    sha256: str | None = None


def parts(relative: str) -> tuple[str, ...]:
    if not isinstance(relative, str) or not relative or "\\" in relative or any(ord(c) < 32 for c in relative):
        raise ValueError("Invalid relative book path")
    path = PurePosixPath(relative)
    if path.is_absolute() or str(path) != relative or any(p in {"", ".", ".."} for p in path.parts):
        raise ValueError("Book path must be a normalized relative path")
    return path.parts


def _int(value, name: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ValueError(f"Invalid {name}")
    return value


def _sha(value) -> str:
    if not isinstance(value, str) or len(value) != 64 or any(c not in "0123456789abcdefABCDEF" for c in value):
        raise ValueError("Invalid expected SHA-256")
    return value.lower()


def records(snapshot: Path, imports: Path) -> list[Record]:
    """Read only the stable snapshot and newline-terminated completion records."""
    admitted: dict[str, Record] = {}

    def add(record: Record):
        parts(record.relative)
        if PurePosixPath(record.relative).name.startswith("~$"):
            return  # Office owner-lock metadata is not the book's document body.
        if PurePosixPath(record.relative).suffix.lower() not in FORMATS:
            return
        previous = admitted.get(record.relative)
        if previous:
            if previous.size != record.size or previous.sha256 and record.sha256 and previous.sha256 != record.sha256:
                raise ValueError(f"Conflicting completed records: {record.relative}")
            record = Record(record.relative, record.size, previous.mtime_ns if previous.mtime_ns is not None else record.mtime_ns,
                            previous.sha256 or record.sha256)
        admitted[record.relative] = record

    data = json.loads(snapshot.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or not isinstance(data.get("files"), list):
        raise ValueError("Snapshot files list missing")
    for row in data["files"]:
        if not isinstance(row, dict):
            raise ValueError("Invalid snapshot record")
        relative = row.get("relative")
        if isinstance(relative, str) and PurePosixPath(relative).suffix.lower() in FORMATS:
            add(Record(relative, _int(row.get("bytes"), "snapshot bytes"), _int(row.get("mtime_ns"), "snapshot mtime"),
                       _sha(row["sha256"]) if row.get("sha256") else None))
    if imports.exists():
        with imports.open("rb") as source:
            for number, line in enumerate(source, 1):
                # The importer may currently be appending. A fragment is not a
                # completed record and must never admit its in-flight PDF.
                if not line.endswith(b"\n"):
                    continue
                if not line.strip():
                    continue
                row = json.loads(line)
                if not isinstance(row, dict):
                    raise ValueError(f"Invalid import record at line {number}")
                if row.get("status") not in COMPLETED or row.get("byte_identity_verified") is not True:
                    continue
                destination = row.get("destination")
                if not isinstance(destination, str) or not destination.startswith("Books/"):
                    raise ValueError(f"Invalid completed destination at line {number}")
                expected = _sha(row.get("sha256"))
                if row.get("source_sha256_unchanged") is not True or row.get("source_mtime_unchanged") is not True:
                    continue
                if row.get("source_kind") == "archive_member":
                    # Container identity and extracted member identity are
                    # separate facts. Never compare a PDF/DOC body to archive SHA.
                    if _sha(row.get("source_member_sha256")) != expected or _sha(row.get("archive_sha256_before")) != _sha(row.get("archive_sha256_after")):
                        continue
                    before_mtime = row.get("archive_mtime_before_ns", row.get("source_mtime_before_ns"))
                    after_mtime = row.get("archive_mtime_after_ns", row.get("source_mtime_after_ns"))
                    if _int(before_mtime, "archive mtime before") != _int(after_mtime, "archive mtime after"):
                        continue
                    if "archive_volumes" in row:
                        volumes = row["archive_volumes"]
                        if not isinstance(volumes, list) or not volumes:
                            raise ValueError("Invalid multipart archive proof")
                        unchanged = True
                        for volume in volumes:
                            if not isinstance(volume, dict):
                                raise ValueError("Invalid archive volume proof")
                            parts(volume.get("source_relative"))
                            _int(volume.get("bytes"), "archive volume bytes")
                            if _sha(volume.get("sha256_before")) != _sha(volume.get("sha256_after")) or _int(volume.get("mtime_before_ns"), "archive volume mtime before") != _int(volume.get("mtime_after_ns"), "archive volume mtime after"):
                                unchanged = False
                        if not unchanged:
                            continue
                elif _sha(row.get("source_hash_after")) != expected:
                    continue
                add(Record(destination[len("Books/"):], _int(row.get("bytes"), "completed bytes"), sha256=expected))
    return [admitted[key] for key in sorted(admitted)]


def open_directory(path: Path, create: bool = False) -> int:
    """Open an absolute directory without following any ancestor symlinks."""
    absolute = path.absolute()
    if ".." in absolute.parts:
        raise ValueError("Directory path contains parent traversal")
    fd = os.open("/", DIR_FLAGS)
    try:
        for part in absolute.parts[1:]:
            if create:
                try:
                    os.mkdir(part, 0o755, dir_fd=fd)
                except FileExistsError:
                    pass
            child = os.open(part, DIR_FLAGS, dir_fd=fd)
            os.close(fd)
            fd = child
        return fd
    except BaseException:
        os.close(fd)
        raise


def _parent(root_fd: int, relative: str, create: bool = False) -> tuple[int, str]:
    components = parts(relative)
    fd = os.dup(root_fd)
    try:
        for part in components[:-1]:
            if create:
                try:
                    os.mkdir(part, 0o755, dir_fd=fd)
                except FileExistsError:
                    pass
            child = os.open(part, DIR_FLAGS, dir_fd=fd)
            os.close(fd)
            fd = child
        return fd, components[-1]
    except BaseException:
        os.close(fd)
        raise


def _regular(fd: int):
    info = os.fstat(fd)
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
        raise ValueError("Book must be a regular file with one directory link")
    return info


def _signature(info) -> tuple[int, ...]:
    return info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns


def _digest(fd: int, destination_fd: int | None = None) -> str:
    os.lseek(fd, 0, os.SEEK_SET)
    digest = hashlib.sha256()
    while True:
        data = os.read(fd, BUFFER)
        if not data:
            break
        digest.update(data)
        if destination_fd is not None:
            view = memoryview(data)
            while view:
                count = os.write(destination_fd, view)
                if count <= 0:
                    raise OSError("Copy write made no progress")
                view = view[count:]
    return digest.hexdigest()


def _stable_source(fd: int, parent_fd: int, name: str, before, digest: str):
    after_hash = _digest(fd)
    after = _regular(fd)
    named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
    if digest != after_hash or _signature(before) != _signature(after) or _signature(before) != _signature(named):
        raise ValueError("Source changed during staging; cached publication withheld")


def _stable_stat(fd: int, parent_fd: int, name: str, before):
    named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
    if _signature(before) != _signature(_regular(fd)) or _signature(before) != _signature(named):
        raise ValueError("Source changed during cached verification")


def format_metadata(fd: int, relative: str) -> dict:
    extension = PurePosixPath(relative).suffix.lower()
    if extension not in FORMATS or PurePosixPath(relative).name.startswith("~$"):
        raise ValueError("Only enumerated supported original book formats may be staged")
    os.lseek(fd, 0, os.SEEK_SET)
    header = os.read(fd, 4096)
    accepted = {".pdf": header.startswith(b"%PDF-"),
                ".doc": header.startswith(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1") or header.startswith(b"{\\rtf"),
                ".docx": header.startswith(b"PK\x03\x04"),
                ".epub": header.startswith(b"PK\x03\x04"),
                ".djvu": header.startswith(b"AT&TFORM")}
    kind, media_type, disposition = FORMATS[extension]
    subtype = {".pdf": "pdf_header", ".docx": "zip_docx_container", ".epub": "zip_epub_container", ".djvu": "djvu_container"}.get(extension)
    encoding = None
    if extension == ".doc":
        if header.startswith(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1"):
            subtype = "ole_document"
        elif header.startswith(b"{\\rtf"):
            subtype = "rtf_document"
        else:
            # Original legacy DOCs sometimes contain text or exported HTML.
            # Detection is bounded; bytes are never decoded/re-encoded for storage.
            # Download-only octet-stream prevents an HTML original being rendered.
            if not header or header.startswith((b"MZ", b"\x7fELF", b"#!")) or any((b < 32 and b not in (9, 10, 11, 12, 13)) or b == 127 for b in header):
                raise ValueError("Invalid .doc binary/control header; not admitted as legacy book text")
            try:
                text = header.decode("utf-8", errors="strict")
                encoding = "utf-8"
            except UnicodeDecodeError:
                try:
                    text = header.decode("cp1252", errors="strict")
                    encoding = "cp1252"
                except UnicodeDecodeError as error:
                    raise ValueError("Invalid .doc text header encoding") from error
            prefix = text.removeprefix("\ufeff").lstrip().lower()
            if not prefix:
                raise ValueError("Empty .doc text header")
            subtype = "html_document_original_doc_extension" if prefix.startswith(("<html", "<!doctype html")) else "plain_text_original_doc_extension"
            media_type = "application/octet-stream"
            accepted[extension] = True
    if not accepted[extension]:
        raise ValueError(f"Invalid {extension} binary header; original bytes are not transformed")
    return {"format": kind, "media_type": media_type, "disposition": disposition,
            "detected_subtype": subtype, "text_encoding": encoding}


def stage_one(source_fd: int, cache_fd: int, record: Record, previous: dict | None = None, proof: dict | None = None) -> dict:
    src_parent, src_name = _parent(source_fd, record.relative)
    dst_parent = src = temp = None
    temporary_name = None
    try:
        src = os.open(src_name, os.O_RDONLY | NOFOLLOW, dir_fd=src_parent)
        before = _regular(src)
        if before.st_size != record.size or record.mtime_ns is not None and before.st_mtime_ns != record.mtime_ns:
            raise ValueError("Source does not match stable snapshot metadata")
        metadata = format_metadata(src, record.relative)
        skip_source_hash = False
        expected = None
        if previous is not None:
            if previous.get("path") != record.relative:
                raise ValueError("Previous admitted path does not match")
            if proof:
                if proof.get("path") != record.relative or _int(proof.get("bytes"), "proof bytes") != record.size:
                    raise ValueError("Stored verification metadata differs from current source record")
                expected = _sha(proof.get("sha256"))
                if previous.get("sha256") and _sha(previous["sha256"]) != expected:
                    raise ValueError("Previous manifest and verification hashes disagree")
                signature = proof.get("source_stat")
                skip_source_hash = isinstance(signature, list) and signature == list(_signature(before))
            elif previous.get("sha256"):
                expected = _sha(previous["sha256"])
            # One-time bootstrap from an expected hash recorded by the completed
            # source importer is allowed only with the stable local snapshot's
            # size/mtime admission. A cache hash alone cannot establish source identity.
            elif record.sha256 and record.mtime_ns is not None:
                expected = record.sha256
                skip_source_hash = True
            if record.sha256 and expected and record.sha256 != expected:
                raise ValueError("Stored expected hash differs from the completed source proof")
        dst_parent, dst_name = _parent(cache_fd, record.relative, create=True)
        try:
            cached = os.open(dst_name, os.O_RDONLY | NOFOLLOW, dir_fd=dst_parent)
        except FileNotFoundError:
            cached = None
        if cached is not None:
            try:
                cache_before = _regular(cached)
                cache_hash = _digest(cached)
                if expected and cache_hash != expected:
                    raise ValueError("Existing cache differs from stored expected SHA-256; no overwrite permitted")
                source_hash = expected if skip_source_hash else _digest(src)
                if record.sha256 and source_hash != record.sha256:
                    raise ValueError("Source SHA-256 differs from completed record")
                if cache_before.st_size != before.st_size or cache_hash != source_hash:
                    raise ValueError("Existing cached PDF differs; no overwrite permitted")
                if _signature(cache_before) != _signature(_regular(cached)) or _signature(cache_before) != _signature(os.stat(dst_name, dir_fd=dst_parent, follow_symlinks=False)):
                    raise ValueError("Cached PDF changed during verification")
                if skip_source_hash:
                    _stable_stat(src, src_parent, src_name, before)
                else:
                    _stable_source(src, src_parent, src_name, before, source_hash)
            finally:
                os.close(cached)
            status = "verified_existing"
        else:
            temporary_name = ".reader-" + uuid.uuid4().hex + ".tmp"
            temp = os.open(temporary_name, os.O_RDWR | os.O_CREAT | os.O_EXCL | NOFOLLOW, 0o600, dir_fd=dst_parent)
            source_hash = _digest(src, temp)
            os.fsync(temp)
            if record.sha256 and source_hash != record.sha256:
                raise ValueError("Source SHA-256 differs from completed record")
            if expected and source_hash != expected:
                raise ValueError("Source differs from the previously stored expected SHA-256")
            # Verify bytes actually stored, not only the hash of the read buffer.
            if os.fstat(temp).st_size != before.st_size or _digest(temp) != source_hash:
                raise ValueError("Staged bytes failed read-back SHA-256 verification")
            _stable_source(src, src_parent, src_name, before, source_hash)
            os.fchmod(temp, 0o444)
            # Atomic no-replace publication. Unlike rename/replace this fails
            # if any competing process created the final destination.
            os.link(temporary_name, dst_name, src_dir_fd=dst_parent, dst_dir_fd=dst_parent, follow_symlinks=False)
            os.unlink(temporary_name, dir_fd=dst_parent)
            temporary_name = None
            os.fsync(dst_parent)
            status = "copied"
        return {"path": record.relative, "filename": src_name, "sha256": source_hash, "bytes": before.st_size, "status": status,
                "source_stat": list(_signature(before)), "source_hash_reused": status == "verified_existing" and skip_source_hash, **metadata}
    finally:
        if temp is not None:
            os.close(temp)
        if temporary_name is not None and dst_parent is not None:
            os.unlink(temporary_name, dir_fd=dst_parent)
        for fd in (src, src_parent, dst_parent):
            if fd is not None:
                os.close(fd)


def read_index(source: Path) -> dict:
    parent = open_directory(source.parent)
    fd = None
    try:
        fd = os.open(source.name, os.O_RDONLY | NOFOLLOW, dir_fd=parent)
        before = _regular(fd)
        if before.st_size > 8 * BUFFER:
            raise ValueError("Private manifest exceeds bounded 8 MiB size")
        data = bytearray()
        while len(data) <= before.st_size:
            chunk = os.read(fd, min(BUFFER, before.st_size + 1 - len(data)))
            if not chunk:
                break
            data.extend(chunk)
        if len(data) != before.st_size or _signature(before) != _signature(_regular(fd)):
            raise ValueError("Private manifest changed during read")
        decoded = json.loads(data)
        if not isinstance(decoded, dict) or decoded.get("version") != 1 or not isinstance(decoded.get("files"), dict):
            raise ValueError("Invalid private manifest")
        if len(decoded["files"]) > 20000:
            raise ValueError("Private manifest exceeds file limit")
        for key, item in decoded["files"].items():
            if not isinstance(item, dict) or not isinstance(item.get("path"), str):
                raise ValueError("Invalid admitted file entry")
            relative = item["path"]
            parts(relative)
            if PurePosixPath(relative).suffix.lower() not in FORMATS or key != hashlib.sha256(relative.encode("utf-8")).hexdigest()[:32]:
                raise ValueError("Invalid admitted file key/path")
        return decoded["files"]
    finally:
        if fd is not None:
            os.close(fd)
        os.close(parent)


def write_index(destination: Path, files: dict[str, dict], mode: int = 0o644):
    parent = open_directory(destination.parent, create=True)
    temporary = ".reader-index-" + uuid.uuid4().hex + ".tmp"
    try:
        try:
            current = os.stat(destination.name, dir_fd=parent, follow_symlinks=False)
            if not stat.S_ISREG(current.st_mode) or current.st_nlink != 1:
                raise ValueError("Reader index cannot replace a symlink or non-regular file")
        except FileNotFoundError:
            pass
        fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | NOFOLLOW, mode, dir_fd=parent)
        os.fchmod(fd, mode)
        with os.fdopen(fd, "w", encoding="utf-8") as output:
            json.dump({"version": 1, "files": files}, output, ensure_ascii=False, sort_keys=True, indent=2)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, destination.name, src_dir_fd=parent, dst_dir_fd=parent)
        os.fsync(parent)
    finally:
        try:
            os.unlink(temporary, dir_fd=parent)
        except FileNotFoundError:
            pass
        os.close(parent)


def stage(source: Path, cache: Path, snapshot: Path, imports: Path, index: Path,
          existing_index: Path | None = None, verified_metadata: Path | None = None) -> dict:
    verified_metadata = verified_metadata or index.with_name("book-cache-verification.json")
    source_absolute, cache_absolute, index_absolute = source.absolute(), cache.absolute(), index.absolute()
    if any(".." in path.parts for path in (source_absolute, cache_absolute, index_absolute)):
        raise ValueError("Configuration path contains parent traversal")
    def within(path: Path, directory: Path) -> bool:
        return path == directory or directory in path.parents
    if within(cache_absolute, source_absolute) or within(source_absolute, cache_absolute) or within(index_absolute, source_absolute) or within(index_absolute, cache_absolute):
        raise ValueError("Source, cache and reader index configuration overlap; no source mutation or cached PDF replacement is permitted")
    proof_absolute = verified_metadata.absolute()
    if ".." in proof_absolute.parts or within(proof_absolute, source_absolute) or within(proof_absolute, cache_absolute) or proof_absolute == index_absolute:
        raise ValueError("Verification metadata configuration overlap")
    selected = records(snapshot, imports)
    old = read_index(existing_index) if existing_index is not None else {}
    proofs = read_index(verified_metadata) if existing_index is not None and verified_metadata.exists() else {}
    source_fd = open_directory(source)
    cache_fd = None
    files, verified, failures = {}, {}, []
    copied = reused = source_hashes_reused = 0
    try:
        cache_fd = open_directory(cache, create=True)
        if os.fstat(source_fd).st_dev == os.fstat(cache_fd).st_dev and os.fstat(source_fd).st_ino == os.fstat(cache_fd).st_ino:
            raise ValueError("Cache cannot be the source Books directory")
        for number, record in enumerate(selected, 1):
            try:
                key = hashlib.sha256(record.relative.encode("utf-8")).hexdigest()[:32]
                result = stage_one(source_fd, cache_fd, record, previous=old.get(key), proof=proofs.get(key) if key in old else None)
                if key in files and files[key]["path"] != record.relative:
                    raise ValueError("Opaque path ID collision")
                files[key] = {field: result[field] for field in ("path", "filename", "sha256", "bytes", "format", "media_type", "disposition", "detected_subtype", "text_encoding")}
                verified[key] = {field: result[field] for field in ("path", "sha256", "bytes", "source_stat")}
                copied += result["status"] == "copied"
                reused += result["status"] == "verified_existing"
                source_hashes_reused += result["source_hash_reused"]
            except (OSError, ValueError) as error:
                failures.append({"path": record.relative, "error": str(error)})
            if number % 25 == 0:
                print(json.dumps({"processed": number, "enumerated": len(selected), "copied": copied, "verified_existing": reused, "withheld": len(failures)}), flush=True)
        write_index(verified_metadata, verified, mode=0o600)
        write_index(index, files)
    finally:
        os.close(source_fd)
        if cache_fd is not None:
            os.close(cache_fd)
    return {"enumerated": len(selected), "indexed": len(files), "copied": copied, "verified_existing": reused,
            "source_hashes_reused": source_hashes_reused, "by_format": {kind: sum(item["format"] == kind for item in files.values()) for kind in sorted({item["format"] for item in files.values()})}, "withheld": failures,
            "basis": "SHA-256 copy identity only; no authorship, full-book review or strategy certification."}


class StagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="vault-reader-test-")
        self.root = Path(self.temporary.name)
        self.source = self.root / "Books"
        self.cache = self.root / "cache"
        self.source.mkdir()
        self.cache.mkdir()
        self.pdf = self.source / "Author" / "Book.pdf"
        self.pdf.parent.mkdir()
        self.payload = b"%PDF-private-copy-test\n" + b"x" * (BUFFER + 17)
        self.pdf.write_bytes(self.payload)
        self.record = Record("Author/Book.pdf", len(self.payload), self.pdf.stat().st_mtime_ns)
        self.snapshot = self.root / "snapshot.json"
        self.snapshot.write_text(json.dumps({"files": [{"relative": self.record.relative, "bytes": self.record.size, "mtime_ns": self.record.mtime_ns}]}))
        self.imports = self.root / "imports.jsonl"
        self.imports.write_text("")
        self.index = self.root / "book-index.json"

    def tearDown(self):
        self.temporary.cleanup()

    def test_copy_and_reuse_preserve_source_and_exact_bytes(self):
        before = _signature(self.pdf.stat())
        first = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        self.assertEqual((first["copied"], first["indexed"]), (1, 1))
        second = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        self.assertEqual((second["copied"], second["verified_existing"]), (0, 1))
        self.assertEqual(self.payload, (self.cache / self.record.relative).read_bytes())
        self.assertEqual(before, _signature(self.pdf.stat()))
        data = json.loads(self.index.read_text())
        key = hashlib.sha256(self.record.relative.encode()).hexdigest()[:32]
        self.assertEqual(data["version"], 1)
        self.assertEqual(set(data["files"]), {key})
        self.assertEqual(data["files"][key]["path"], self.record.relative)
        self.assertEqual(data["files"][key]["filename"], "Book.pdf")
        self.assertEqual(data["files"][key]["sha256"], hashlib.sha256(self.payload).hexdigest())
        self.assertEqual(data["files"][key]["bytes"], len(self.payload))
        self.assertEqual(data["files"][key]["disposition"], "inline")

    def test_existing_mismatched_cache_is_untouched_and_unindexed(self):
        target = self.cache / self.record.relative
        target.parent.mkdir()
        target.write_bytes(b"different cached PDF")
        result = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        self.assertEqual(result["indexed"], 0)
        self.assertEqual(len(result["withheld"]), 1)
        self.assertEqual(target.read_bytes(), b"different cached PDF")

    def test_changed_source_and_expected_hash_mismatch_are_withheld(self):
        self.pdf.write_bytes(b"new bytes")
        result = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        self.assertEqual(result["indexed"], 0)
        self.assertFalse((self.cache / self.record.relative).exists())
        self.pdf.write_bytes(self.payload)
        source_fd, cache_fd = open_directory(self.source), open_directory(self.cache)
        try:
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                stage_one(source_fd, cache_fd, Record(self.record.relative, len(self.payload), sha256="0" * 64))
        finally:
            os.close(source_fd)
            os.close(cache_fd)
        self.assertFalse((self.cache / self.record.relative).exists())
        self.assertFalse(list(self.cache.rglob("*.tmp")))

    def test_inflight_unenumerated_and_nonpdf_files_are_not_copied(self):
        (self.source / "not-enumerated.pdf").write_bytes(b"%PDF-never-copy")
        (self.source / "archive.zip").write_bytes(b"archive")
        incomplete = {"status": "copied", "destination": "Books/not-enumerated.pdf", "byte_identity_verified": True}
        self.imports.write_text(json.dumps(incomplete))  # not a completed newline-terminated record
        result = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        self.assertEqual(result["indexed"], 1)
        self.assertFalse((self.cache / "not-enumerated.pdf").exists())
        self.assertFalse((self.cache / "archive.zip").exists())

    def test_completed_record_uses_expected_hash_and_not_research_source_mtime(self):
        self.snapshot.write_text('{"files":[]}')
        row = {"status": "copied", "destination": "Books/Author/Book.pdf", "bytes": len(self.payload),
               "byte_identity_verified": True, "sha256": hashlib.sha256(self.payload).hexdigest(),
               "source_hash_after": hashlib.sha256(self.payload).hexdigest(), "source_mtime_before_ns": 1,
               "source_mtime_unchanged": True, "source_sha256_unchanged": True}
        for status in ["copied", "reused_local_identical_sha256", "already_present_identical"]:
            row["status"] = status
            self.imports.write_text(json.dumps(row) + "\n")
            self.assertEqual(stage(self.source, self.cache, self.snapshot, self.imports, self.index)["indexed"], 1)

    def test_traversal_and_source_cache_symlinks_are_rejected(self):
        for value in ["../x.pdf", "/x.pdf", "Author/../x.pdf", "Author//x.pdf", "Author/./x.pdf", "Author\\x.pdf"]:
            with self.assertRaises(ValueError):
                parts(value)
        outside = self.root / "outside"
        outside.mkdir()
        (self.cache / "Author").symlink_to(outside, target_is_directory=True)
        self.assertEqual(stage(self.source, self.cache, self.snapshot, self.imports, self.index)["indexed"], 0)
        self.assertEqual(list(outside.iterdir()), [])
        (self.cache / "Author").unlink()
        self.pdf.unlink()
        outside_pdf = outside / "outside.pdf"
        outside_pdf.write_bytes(self.payload)
        self.pdf.symlink_to(outside_pdf)
        self.assertEqual(stage(self.source, self.cache, self.snapshot, self.imports, self.index)["indexed"], 0)

    def test_source_changed_during_stream_is_not_published(self):
        original = _digest
        calls = 0
        def racing_digest(fd, destination_fd=None):
            nonlocal calls
            result = original(fd, destination_fd)
            calls += 1
            if calls == 1:
                self.pdf.write_bytes(b"%PDF-concurrent-change")
            return result
        with mock.patch(__name__ + "._digest", side_effect=racing_digest):
            result = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        self.assertEqual(result["indexed"], 0)
        self.assertFalse((self.cache / self.record.relative).exists())
        self.assertFalse(list(self.cache.rglob("*.tmp")))

    def test_symlinked_index_is_not_replaced(self):
        outside = self.root / "outside-index.json"
        outside.write_text("keep")
        self.index.symlink_to(outside)
        with self.assertRaises(ValueError):
            write_index(self.index, {})
        self.assertEqual(outside.read_text(), "keep")

    def test_configuration_cannot_write_under_source_or_replace_cached_pdf_with_index(self):
        for cache, index in [(self.source / "reader-cache", self.index),
                             (self.cache, self.source / "reader-index.json"),
                             (self.cache, self.cache / self.record.relative)]:
            with self.assertRaisesRegex(ValueError, "overlap"):
                stage(self.source, cache, self.snapshot, self.imports, index)
        self.assertFalse((self.source / "reader-cache").exists())
        self.assertFalse((self.source / "reader-index.json").exists())
        self.assertFalse((self.cache / self.record.relative).exists())
        self.assertEqual(self.pdf.read_bytes(), self.payload)

    def test_pdf_header_must_match_reader_without_changing_the_source(self):
        for payload in [b"renamed binary with .pdf extension", b"prefix\n%PDF-1.4\nbody"]:
            self.pdf.write_bytes(payload)
            before = _signature(self.pdf.stat())
            self.snapshot.write_text(json.dumps({"files": [{"relative": self.record.relative, "bytes": len(payload), "mtime_ns": self.pdf.stat().st_mtime_ns}]}))
            report = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
            self.assertEqual(report["indexed"], 0)
            self.assertIn("header", report["withheld"][0]["error"])
            self.assertEqual(self.pdf.read_bytes(), payload)
            self.assertEqual(_signature(self.pdf.stat()), before)
            self.assertFalse((self.cache / self.record.relative).exists())

    def test_original_attachments_preserve_bytes_and_disclose_download_format(self):
        examples = {"original.doc": b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1" + b"ole-original",
                    "text.doc": b"{\\rtf1 original}", "word.docx": b"PK\x03\x04original-docx",
                    "book.epub": b"PK\x03\x04original-epub", "scan.djvu": b"AT&TFORM\x00\x00\x00\x04DJVU"}
        entries = []
        for filename, payload in examples.items():
            target = self.source / filename
            target.write_bytes(payload)
            entries.append({"relative": filename, "bytes": len(payload), "mtime_ns": target.stat().st_mtime_ns})
        self.snapshot.write_text(json.dumps({"files": entries}))
        result = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        self.assertEqual(result["indexed"], len(examples))
        for filename, payload in examples.items():
            self.assertEqual((self.cache / filename).read_bytes(), payload)
        items = list(json.loads(self.index.read_text())["files"].values())
        self.assertTrue(all(item["disposition"] == "attachment" for item in items))
        self.assertEqual({item["format"] for item in items}, {"doc", "docx", "epub", "djvu"})

    def test_bad_attachment_signatures_and_traversal_are_not_admitted(self):
        entries = []
        for filename in ["wrong.doc", "wrong.docx", "wrong.djvu", "wrong.epub"]:
            target = self.source / filename
            target.write_bytes(b"MZ-renamed executable\x00binary")
            entries.append({"relative": filename, "bytes": target.stat().st_size, "mtime_ns": target.stat().st_mtime_ns})
        self.snapshot.write_text(json.dumps({"files": entries}))
        result = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        self.assertEqual(result["indexed"], 0)
        self.assertEqual(len(result["withheld"]), 4)
        self.assertTrue(all("header" in row["error"] for row in result["withheld"]))
        self.snapshot.write_text(json.dumps({"files": [{"relative": "../escape.doc", "bytes": 0, "mtime_ns": 0}]}))
        with self.assertRaises(ValueError):
            stage(self.source, self.cache, self.snapshot, self.imports, self.index)

    def test_legacy_text_and_html_docs_are_original_downloads_never_inline(self):
        examples = {"utf8.doc": "A legacy strategy note with café".encode("utf-8"),
                    "cp1252.doc": b"A legacy book\xa0with whitespace\r\n",
                    "html.doc": b" \r\n<!DOCTYPE HTML><html><script>no inline execution</script></html>"}
        entries = []
        for filename, payload in examples.items():
            target = self.source / filename
            target.write_bytes(payload)
            entries.append({"relative": filename, "bytes": len(payload), "mtime_ns": target.stat().st_mtime_ns})
        self.snapshot.write_text(json.dumps({"files": entries}))
        report = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        self.assertEqual(report["indexed"], len(examples))
        items = {item["filename"]: item for item in json.loads(self.index.read_text())["files"].values()}
        for filename, payload in examples.items():
            self.assertEqual((self.cache / filename).read_bytes(), payload)
            self.assertEqual(items[filename]["disposition"], "attachment")
            self.assertEqual(items[filename]["media_type"], "application/octet-stream")
        self.assertEqual(items["html.doc"]["detected_subtype"], "html_document_original_doc_extension")
        self.assertEqual(items["cp1252.doc"]["text_encoding"], "cp1252")

    def test_legacy_doc_binary_controls_undefined_encoding_and_ownerlocks_are_excluded(self):
        for payload in [b"MZ-not-a-book", b"body\x00rest", b"body\x01rest", b"body\x7frest", b"body\x81rest"]:
            target = self.source / "bad.doc"
            target.write_bytes(payload)
            self.snapshot.write_text(json.dumps({"files": [{"relative": "bad.doc", "bytes": len(payload), "mtime_ns": target.stat().st_mtime_ns}]}))
            result = stage(self.source, self.cache, self.snapshot, self.imports, self.index)
            self.assertEqual(result["indexed"], 0)
            self.assertEqual(len(result["withheld"]), 1)
        lock = self.source / "~$owner.doc"
        lock.write_bytes(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1owner-lock")
        self.snapshot.write_text(json.dumps({"files": [{"relative": lock.name, "bytes": lock.stat().st_size, "mtime_ns": lock.stat().st_mtime_ns}]}))
        self.assertEqual(records(self.snapshot, self.imports), [])

    def test_incremental_verified_cache_is_hashed_but_source_is_not_reread(self):
        proof = self.root / "book-cache-verification.json"
        stage(self.source, self.cache, self.snapshot, self.imports, self.index, verified_metadata=proof)
        original = _digest
        source_identity = (self.pdf.stat().st_dev, self.pdf.stat().st_ino)
        def forbid_source_hash(fd, destination_fd=None):
            info = os.fstat(fd)
            self.assertNotEqual((info.st_dev, info.st_ino), source_identity, "Incremental run must use the recorded verified source signature")
            return original(fd, destination_fd)
        with mock.patch(__name__ + "._digest", side_effect=forbid_source_hash):
            result = stage(self.source, self.cache, self.snapshot, self.imports, self.index, existing_index=self.index, verified_metadata=proof)
        self.assertEqual(result["verified_existing"], 1)
        self.assertEqual(result["source_hashes_reused"], 1)

    def test_incremental_tampered_cache_cannot_invent_a_new_expected_hash(self):
        proof = self.root / "book-cache-verification.json"
        stage(self.source, self.cache, self.snapshot, self.imports, self.index, verified_metadata=proof)
        cached = self.cache / self.record.relative
        cached.chmod(0o644)
        cached.write_bytes(self.payload[:-1] + b"y")
        cached.chmod(0o444)
        result = stage(self.source, self.cache, self.snapshot, self.imports, self.index, existing_index=self.index, verified_metadata=proof)
        self.assertEqual(result["indexed"], 0)
        self.assertEqual(len(result["withheld"]), 1)
        self.assertIn("expected", result["withheld"][0]["error"])
        self.assertEqual(cached.read_bytes(), self.payload[:-1] + b"y")

    def test_old_manifest_without_hashes_requires_one_source_bootstrap(self):
        stage(self.source, self.cache, self.snapshot, self.imports, self.index)
        data = json.loads(self.index.read_text())
        for item in data["files"].values():
            for key in ["sha256", "bytes"]:
                item.pop(key, None)
        self.index.write_text(json.dumps(data))
        proof = self.root / "book-cache-verification.json"
        proof.unlink(missing_ok=True)
        original = _digest
        source_hash_calls = 0
        source_identity = (self.pdf.stat().st_dev, self.pdf.stat().st_ino)
        def count_source_hash(fd, destination_fd=None):
            nonlocal source_hash_calls
            info = os.fstat(fd)
            source_hash_calls += (info.st_dev, info.st_ino) == source_identity
            return original(fd, destination_fd)
        with mock.patch(__name__ + "._digest", side_effect=count_source_hash):
            result = stage(self.source, self.cache, self.snapshot, self.imports, self.index, existing_index=self.index, verified_metadata=proof)
        self.assertEqual(result["indexed"], 1)
        self.assertGreaterEqual(source_hash_calls, 2)
        self.assertTrue(proof.exists())

    def test_archive_member_hash_is_distinct_from_unchanged_container_hash(self):
        self.snapshot.write_text('{"files":[]}')
        body = hashlib.sha256(self.payload).hexdigest()
        row = {"status": "copied", "destination": "Books/Author/Book.pdf", "bytes": len(self.payload),
               "source_kind": "archive_member", "sha256": body, "source_member_sha256": body,
               "byte_identity_verified": True, "source_sha256_unchanged": True, "source_mtime_unchanged": True,
               "archive_sha256_before": "a" * 64, "archive_sha256_after": "a" * 64,
               "archive_mtime_before_ns": 1, "archive_mtime_after_ns": 1,
               "source_hash_after": "a" * 64}
        self.imports.write_text(json.dumps(row) + "\n")
        selected = records(self.snapshot, self.imports)
        self.assertEqual(len(selected), 1)
        self.assertEqual(selected[0].sha256, body)
        self.assertEqual(stage(self.source, self.cache, self.snapshot, self.imports, self.index)["indexed"], 1)
        row["archive_sha256_after"] = "b" * 64
        self.imports.write_text(json.dumps(row) + "\n")
        self.assertEqual(records(self.snapshot, self.imports), [])
        row["archive_sha256_after"] = "a" * 64
        row["source_member_sha256"] = "c" * 64
        self.imports.write_text(json.dumps(row) + "\n")
        self.assertEqual(records(self.snapshot, self.imports), [])
        row["source_member_sha256"] = body
        row["archive_volumes"] = [{"source_relative": "archive.part1.rar", "bytes": 123,
                                   "sha256_before": "d" * 64, "sha256_after": "e" * 64,
                                   "mtime_before_ns": 1, "mtime_after_ns": 1}]
        self.imports.write_text(json.dumps(row) + "\n")
        self.assertEqual(records(self.snapshot, self.imports), [])
        row["archive_volumes"][0]["sha256_after"] = "d" * 64
        self.imports.write_text(json.dumps(row) + "\n")
        self.assertEqual(len(records(self.snapshot, self.imports)), 1)


def main() -> int:
    if "--self-test" in sys.argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(StagingTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    repository = Path(__file__).resolve().parents[3]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=repository / "Books")
    parser.add_argument("--snapshot", type=Path, default=repository / "library/source-vault/local-files-2026-10-13.json")
    parser.add_argument("--imports", type=Path, default=repository / "library/source-vault/batch-import-2026-10-13.jsonl")
    parser.add_argument("--cache", type=Path, default=DEFAULT_CACHE)
    parser.add_argument("--index", type=Path, default=DEFAULT_INDEX)
    parser.add_argument("--existing-index", type=Path, help="Merge previously admitted files after cache hash/source provenance checks")
    parser.add_argument("--verified-metadata", type=Path, help="Private per-file hash/stat proof; defaults beside index")
    args = parser.parse_args()
    try:
        report = stage(args.source, args.cache, args.snapshot, args.imports, args.index,
                       existing_index=args.existing_index, verified_metadata=args.verified_metadata)
    except (OSError, ValueError) as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 1
    print(json.dumps(report, ensure_ascii=False), flush=True)
    return 2 if report["withheld"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
