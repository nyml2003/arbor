import { render } from "solid-js/web";
import { App } from "./app";
import "./style.css";

const root = document.querySelector("#app");

if (!root) {
  throw new Error("Settings root element is missing.");
}

render(() => <App />, root);
