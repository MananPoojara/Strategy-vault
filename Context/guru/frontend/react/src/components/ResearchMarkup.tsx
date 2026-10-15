import { createElement, Fragment, type ReactNode } from "react";
import parse, {
  attributesToProps,
  domToReact,
  Element,
  type DOMNode,
  type HTMLReactParserOptions,
} from "html-react-parser";
/** Compatibility for existing source/rule presenters. Creates React nodes, never
 * executes inline HTML or evaluates handler strings. Chart widgets own only their
 * empty container children; the containing page remounts between result snapshots.
 * New pages use JSX directly. This is not an arbitrary user-HTML renderer. */
export function ResearchMarkup({ html, slots = {} }: {
  html: string;
  /** Components supplied by the owning React page, never by source HTML. */
  slots?: Readonly<Record<string, ReactNode>>;
}) {
  const options: HTMLReactParserOptions = {
    replace(node, index) {
      if (!(node instanceof Element)) return;
      if (
        [
          "script",
          "iframe",
          "object",
          "embed",
          "base",
          "link",
          "style",
        ].includes(node.name)
      )
        return createElement(Fragment, { key: index });
      const attributes = { ...node.attribs };
      for (const key of Object.keys(attributes)) {
        if (/^on/i.test(key)) delete attributes[key];
        if (
          ["href", "src", "action", "formaction", "xlink:href"].includes(key) &&
          /^\s*(javascript|vbscript|data):/i.test(attributes[key])
        )
          delete attributes[key];
      }
      const props: Record<string, unknown> = attributesToProps(attributes);
      const slot = attributes["data-react-slot"];
      if (
        slot &&
        ["span", "div"].includes(node.name) &&
        Object.hasOwn(slots, slot)
      ) {
        return createElement(node.name, { ...props, key: index }, slots[slot]);
      }
      if (node.name === "input" || node.name === "textarea") {
        if ("value" in props) {
          props.defaultValue = props.value;
          delete props.value;
        }
        if ("checked" in props) {
          props.defaultChecked = props.checked;
          delete props.checked;
        }
      }
      if (node.name === "select") {
        const selected = (node.children as DOMNode[]).flatMap((child) =>
          child instanceof Element
            ? child.name === "option" && "selected" in child.attribs
              ? [child.attribs.value || ""]
              : child.children
                  .filter(
                    (x) =>
                      x instanceof Element &&
                      x.name === "option" &&
                      "selected" in x.attribs,
                  )
                  .map((x) => (x as Element).attribs.value)
            : [],
        );
        if (selected.length)
          props.defaultValue = props.multiple ? selected : selected[0];
      }
      if (node.name === "option") delete props.selected;
      const onclick = node.attribs.onclick;
      if (onclick === "event.stopPropagation()")
        props.onClick = (event: { stopPropagation: () => void }) =>
          event.stopPropagation();
      else if (onclick === "closeGuru()")
        props.onClick = () => window.VaultRuntime?.closeAuthor();
      else if (onclick?.match(/^openGuru\('[a-zA-Z0-9_-]+', event\)$/)) {
        const id = onclick.match(/^openGuru\('([a-zA-Z0-9_-]+)'/)![1];
        props.onClick = (event: Event) =>
          window.VaultRuntime?.openAuthor(id, event);
      }
      if (node.name === "textarea") {
        props.defaultValue ??= node.children
          .map((child) => (child.type === "text" ? child.data : ""))
          .join("");
        return createElement("textarea", { ...props, key: index });
      }
      const children = domToReact(
        node.children as DOMNode[],
        options,
      ) as ReactNode;
      const voidTag = [
        "area",
        "br",
        "col",
        "hr",
        "img",
        "input",
        "meta",
        "param",
        "source",
        "track",
        "wbr",
      ].includes(node.name);
      return createElement(
        node.name,
        { ...props, key: index },
        ...(voidTag ? [] : [children]),
      );
    },
  };
  return <>{parse(html, options)}</>;
}
