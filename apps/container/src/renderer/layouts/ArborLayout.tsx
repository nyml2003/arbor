import type { JSX } from "solid-js";
import styles from "./layout.module.css";

interface ArborLayoutProps {
  sidebar: JSX.Element;
  children: JSX.Element;
}

export function ArborLayout(props: ArborLayoutProps) {
  return (
    <div class={styles["layout"]}>
      <aside class={styles["sidebar"]}>{props.sidebar}</aside>
      <div class={styles["content"]}>{props.children}</div>
    </div>
  );
}
