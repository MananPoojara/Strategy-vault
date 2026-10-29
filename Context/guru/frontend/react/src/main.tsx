import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import "./styles.css";
import { startSmoothScroll } from "./app/smooth-scroll";
const root = document.getElementById("root");
if (!root) throw new Error("Application root is missing.");
createRoot(root).render(<App />);
startSmoothScroll();
