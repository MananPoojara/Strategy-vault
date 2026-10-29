import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { IndicatorSearch } from "../src/features/lab/OperandEditor";
import { catalogue7Fixture as cat, mount, unmount } from "./fixtures/lab";

const originalShow = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "showPopover");
const originalHide = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "hidePopover");
const show = vi.fn(), hide = vi.fn();
let rect = { left:300, right:380, top:100, bottom:140, width:80, height:40, x:300, y:100, toJSON(){} };
beforeEach(() => {
  vi.stubGlobal("innerWidth",390); vi.stubGlobal("innerHeight",844);
  rect = { ...rect, left:300, right:380, top:100, bottom:140, width:80, height:40, x:300, y:100 };
  vi.spyOn(HTMLButtonElement.prototype,"getBoundingClientRect").mockImplementation(() => rect);
  // jsdom does not implement the native top layer; observe the browser API calls.
  Object.defineProperty(HTMLElement.prototype,"showPopover",{configurable:true,value:show});
  Object.defineProperty(HTMLElement.prototype,"hidePopover",{configurable:true,value:hide});
  show.mockClear(); hide.mockClear();
});
afterEach(async () => {
  await unmount(); vi.restoreAllMocks(); vi.unstubAllGlobals();
  if (originalShow) Object.defineProperty(HTMLElement.prototype,"showPopover",originalShow); else delete (HTMLElement.prototype as { showPopover?: unknown }).showPopover;
  if (originalHide) Object.defineProperty(HTMLElement.prototype,"hidePopover",originalHide); else delete (HTMLElement.prototype as { hidePopover?: unknown }).hidePopover;
});
async function open() {
  const host = await mount(<IndicatorSearch ind={cat.indicators.find(i=>i.id==="rsi")!} computed={cat.indicators} groups={[...new Set(cat.indicators.map(i=>i.category))]} onPick={vi.fn()} />);
  await act(async()=>host.querySelector<HTMLButtonElement>("button")!.click());
  return {host,popup:host.querySelector<HTMLElement>(".lab-ind-pop")!,search:host.querySelector<HTMLInputElement>('input[type="search"]')!};
}

it("opens in the native top layer, fits a narrow viewport and focuses search", async () => {
  const {popup,search} = await open();
  expect(popup.getAttribute("popover")).toBe("auto");
  expect(show).toHaveBeenCalledTimes(1);
  expect(popup.style.position).toBe("fixed");
  expect(popup.style.left).toBe("16px");
  expect(popup.style.width).toBe("358px");
  expect(popup.style.top).toBe("148px");
  expect(popup.style.maxHeight).toBe("420px");
  expect(document.activeElement).toBe(search);
});

it("opens above a low anchor, retains list scrolling and closes when the viewport moves", async () => {
  vi.stubGlobal("innerHeight",300);
  rect={...rect,top:245,bottom:285,y:245};
  const {host,popup} = await open();
  expect(popup.style.top).toBe("237px");
  expect(popup.style.transform).toBe("translateY(-100%)");
  expect(popup.style.maxHeight).toBe("221px");
  await act(async()=>popup.querySelector("ul")!.dispatchEvent(new Event("scroll")));
  expect(host.querySelector(".lab-ind-pop")).toBe(popup);
  await act(async()=>window.dispatchEvent(new Event("resize")));
  expect(host.querySelector(".lab-ind-pop")).toBeNull();
  expect(host.querySelector("button")!.getAttribute("aria-expanded")).toBe("false");
});

it("native light dismissal closes the React search state", async () => {
  const {host,popup} = await open();
  const event=new Event("toggle");
  Object.defineProperties(event,{newState:{value:"closed"},oldState:{value:"open"}});
  await act(async()=>popup.dispatchEvent(event));
  expect(host.querySelector(".lab-ind-pop")).toBeNull();
  expect(host.querySelector("button")!.getAttribute("aria-expanded")).toBe("false");
});
