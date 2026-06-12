import type { JSX } from "solid-js";
import styles from "./layout.module.css";

interface ArborLayoutProps {
  sidebar: JSX.Element;
  children: JSX.Element;
}

export function ArborLayout(props: ArborLayoutProps) {
  return (
    <div class={styles.layout}>
      {props.sidebar}
      <div class={styles.content}>{props.children}</div>
    </div>
  );
}
