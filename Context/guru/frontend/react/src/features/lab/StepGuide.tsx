/** "Step n of 4" card above each strategy step: what the step is for and how to do it, with tips a returning user can hide. */
import { useState } from "react";
import "./simple.css";

const KEY = "lab.tips";
const stored = () => { try { return localStorage.getItem(KEY) !== "hidden"; } catch { return true; } };

export function StepGuide({ n, of, step }: { n: number; of: number; step: { label: string; hint: string; tips: readonly string[] } }) {
  const [tips, setTips] = useState(stored);
  const toggle = () => {
    setTips(!tips);
    try { localStorage.setItem(KEY, tips ? "hidden" : "shown"); } catch { /* private window: the choice lasts this visit */ }
  };
  return <div className="lab-guide" data-lab-step-hint>
    <div className="lab-guide-h">
      <span className="lab-guide-n">Step {n} of {of}</span>
      <b>{step.label}</b>
      <span className="lab-guide-hint">{step.hint}</span>
      <button type="button" className="dbtn sm ghost lab-guide-toggle" aria-expanded={tips} onClick={toggle}>{tips ? "Hide tips" : "How do I use this step?"}</button>
    </div>
    {tips && <ol className="lab-guide-tips">{step.tips.map((t) => <li key={t}>{t}</li>)}</ol>}
  </div>;
}
