import "./lib/styles/tokens.css";
import { mount } from "svelte";
import App from "./lib/components/App.svelte";

const target = document.getElementById("app");
if (!target) throw new Error("missing #app root element");

export default mount(App, { target });
