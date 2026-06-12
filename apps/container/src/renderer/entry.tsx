import { render } from "solid-js/web";
import { App } from "./App";
import "./styles/tokens.css";
import "./styles/global.css";

document.documentElement.setAttribute("data-theme", "dark");

const root = document.getElementById("app");
if (!root) throw new Error("Root element #app not found");
render(() => <App />, root);
