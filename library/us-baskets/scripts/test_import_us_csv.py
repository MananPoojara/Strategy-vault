import csv
from pathlib import Path
import tempfile
import unittest
import json
import datetime as dt
from unittest.mock import patch
from import_us_csv import date_format, parse_date, validate_csv, classify_security, validate_source_path, run, digest, stage_source


HEADER=['Date','Symbol','Security Name','Open','High','Low','Close','Volume']


class USCSVTests(unittest.TestCase):
    def test_date_format_is_proven_and_ambiguous_slash_is_withheld(self):
        self.assertEqual(date_format(['2020-09-11','2020-09-14']),'iso')
        self.assertEqual(date_format(['20200911','20200914']),'compact')
        self.assertEqual(date_format(['01/02/2020','13/02/2020']),'dmy')
        self.assertEqual(date_format(['02/01/2020','02/13/2020']),'mdy')
        self.assertEqual(date_format(['29-02-1988','01-03-1988']),'dmy_dash')
        self.assertEqual(date_format(['02-01-2020','02-13-2020']),'mdy_dash')
        for dates in [['01/02/2020','02/03/2020'],['13/02/2020','02/13/2020'],['20200911','2020-09-14'],['11-11-2015','12-11-2015']]:
            with self.assertRaises(ValueError):date_format(dates)
        self.assertEqual(parse_date('13/02/2020','dmy').isoformat(),'2020-02-13')
        self.assertEqual(parse_date('29-02-1988','dmy_dash').isoformat(),'1988-02-29')

    def test_strict_stock_rows_preserve_delisted_identity_zero_volume_and_decimal_strings(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'AABA-201910.csv'; out=Path(d)/'normalized.csv'
            with p.open('w',newline='') as stream:
                w=csv.writer(stream);w.writerow(HEADER)
                w.writerow(['20190927','AABA-201910','Altaba Inc Common','10.123456789','11','9','10.5','0'])
                w.writerow(['20190930','AABA-201910','Altaba Inc Common','10.5','11.5','10','11','123'])
            r=validate_csv(p,'AABA-201910',out)
            self.assertEqual(r['bars'],2);self.assertEqual(r['first'],'2019-09-27')
            self.assertEqual(r['date_format'],'compact');self.assertTrue(r['has_volume'])
            with out.open() as stream:rows=list(csv.DictReader(stream))
            self.assertEqual(rows[0]['open'],'10.123456789');self.assertEqual(rows[0]['volume'],'0')

    def test_day_month_dash_evidence_requires_full_file_consistency_beyond_probe(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'TEST.csv';out=Path(d)/'out.csv';dates=[dt.date(1988,2,29)+dt.timedelta(days=i) for i in range(260)]
            def write(mixed=False):
                with p.open('w',newline='') as s:
                    w=csv.writer(s);w.writerow(HEADER)
                    for i,date in enumerate(dates):w.writerow([date.isoformat() if mixed and i==259 else date.strftime('%d-%m-%Y'),'TEST','Test Common','10','11','9','10','1'])
            write();r=validate_csv(p,'TEST',out)
            self.assertEqual(r['date_format'],'dmy_dash');self.assertEqual(r['bars'],260)
            self.assertEqual(r['date_format_evidence']['order_witness']['raw_date'],'29-02-1988')
            self.assertEqual(r['date_format_evidence']['checked_records'],260)
            out.unlink();write(mixed=True)
            with self.assertRaisesRegex(ValueError,'proven date format'):validate_csv(p,'TEST',out)
            self.assertFalse(out.exists())

    def test_malformed_originals_are_not_repaired_or_admitted(self):
        cases=[['2020-09-11','OTHER','Test Common','10','11','9','10','1'],
               ['2020-09-11','TEST','Test Common','10','9','8','10','1'],
               ['2020-09-11','TEST','Test Common','10','11','9','NaN','1'],
               ['2020-09-11','TEST','Test Common','10','11','9','10','-1']]
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'TEST.csv'
            for bad in cases:
                with p.open('w',newline='') as stream:w=csv.writer(stream);w.writerow(HEADER);w.writerow(bad)
                with self.assertRaises(ValueError):validate_csv(p,'TEST',Path(d)/'out.csv')
            valid=['2020-09-11','TEST','Test Common','10','11','9','10','1']
            with p.open('w',newline='') as stream:w=csv.writer(stream);w.writerow(HEADER);w.writerow(valid);w.writerow(valid)
            with self.assertRaises(ValueError):validate_csv(p,'TEST',Path(d)/'out.csv')

    def test_gap_is_reported_without_fill_or_unproved_calendar_exclusion(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'TEST.csv'
            with p.open('w',newline='') as stream:
                w=csv.writer(stream);w.writerow(HEADER)
                for date in ['2020-09-01','2020-09-15']:w.writerow([date,'TEST','Test Common','10','11','9','10','1'])
            r=validate_csv(p,'TEST',Path(d)/'out.csv')
            self.assertEqual(r['bars'],2)
            self.assertEqual(r['observed_calendar_gap_count'],1)
            self.assertEqual(r['gaps'][0]['calendar_days'],14)
            self.assertEqual(r['calendar_completeness'],'not_independently_verified')
            with (Path(d)/'out.csv').open() as stream:rows=list(csv.DictReader(stream))
            self.assertEqual([x['date'] for x in rows],['2020-09-01','2020-09-15'])

    def test_known_indices_etfs_and_unsafe_source_paths_are_rejected(self):
        for ticker,name in [('$SPX','S&P 500 Index'),('SPY','SPDR S&P 500 ETF Trust'),('QQQ','Invesco QQQ Trust')]:
            with self.assertRaises(ValueError):classify_security(ticker,name)
        self.assertEqual(classify_security('AAPL','Apple Inc Common'),'individual_stock')
        self.assertEqual(classify_security('IXOG','Index Oil And Gas Inc Common'),'individual_stock')
        with tempfile.TemporaryDirectory() as d:
            root=Path(d)/'source';root.mkdir();outside=Path(d)/'OUT.csv';outside.write_text('original')
            with self.assertRaises(ValueError):validate_source_path(root,'../OUT.csv')

    def test_source_hash_change_prevents_admission(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'source.csv';p.write_bytes(b'original source');s=p.stat();before=p.read_bytes()
            with patch('import_us_csv.digest',return_value='0'*64):
                with self.assertRaisesRegex(ValueError,'externally changed'):
                    stage_source(p,Path(d)/'copy.csv',{'bytes':s.st_size,'mtime_ns':s.st_mtime_ns})
            self.assertEqual(p.read_bytes(),before)

    def test_full_serial_import_retains_invalid_original_but_exports_only_verified_prices(self):
        with tempfile.TemporaryDirectory() as d:
            base=Path(d);source=base/'source';cache=base/'cache';artifact=base/'artifacts';source.mkdir()
            folder=source/'norgate data 8SEP2020/S&P 500';folder.mkdir(parents=True)
            files=[]
            for ticker,high in [('GOOD','11'),('BAD','8')]:
                p=folder/(ticker+'.csv')
                with p.open('w',newline='') as stream:
                    w=csv.writer(stream);w.writerow(HEADER);w.writerow(['2020-09-11',ticker,'Company Common','10',high,'9','10','0'])
                s=p.stat();files.append({'ticker':ticker,'source_relative':str(p.relative_to(source)),'bytes':s.st_size,'mtime_ns':s.st_mtime_ns})
            manifest={'total_source_bytes':sum(f['bytes'] for f in files),'total_source_file_locations':2,'unique_raw_symbol_identities':2,
                      'baskets':[{'id':'us_sp500','name':'S&P 500 Current','family':'sp500','universe':'current','mapped_index':'SPX',
                                  'membership_kind':'current_snapshot','membership_source':'norgate data 8SEP2020/S&P 500',
                                  'membership_source_sha256':'a'*64,'raw_symbol_count':2,'files':files}]}
            before={p.name:(p.read_bytes(),p.stat().st_mtime_ns) for p in folder.iterdir()}
            run(manifest,source,cache,artifact)
            records=list(map(json.loads,(cache/'import.jsonl').read_text().splitlines()))
            self.assertEqual([r['status'] for r in records],['ready','withheld'])
            with (cache/'index.csv').open() as stream:rows=list(csv.DictReader(stream))
            self.assertEqual(len(rows),1);self.assertEqual(rows[0]['ticker'],'GOOD')
            self.assertEqual(rows[0]['trusted_market'],'us');self.assertEqual(rows[0]['has_volume'],'false')
            self.assertEqual(digest(cache/rows[0]['path']),rows[0]['csv_sha256'])
            self.assertEqual(digest(cache/records[1]['original_copy_path']),records[1]['source_sha256'])
            self.assertEqual(before,{p.name:(p.read_bytes(),p.stat().st_mtime_ns) for p in folder.iterdir()})
            run(manifest,source,cache,artifact)
            self.assertEqual(len((cache/'import.jsonl').read_text().splitlines()),2)


if __name__=='__main__':unittest.main()
