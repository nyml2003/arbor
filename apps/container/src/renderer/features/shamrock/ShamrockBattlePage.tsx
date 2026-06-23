import { createMemo, createSignal, For } from "solid-js";
import { demoBattle, hpRatio } from "./demo-battle";
import { ShamrockScene } from "./ShamrockScene";
import type { BattleActionView, BattleSideView, ElementType } from "./types";
import styles from "./ShamrockBattlePage.module.css";

export function ShamrockBattlePage() {
  const [selectedActionIndex, setSelectedActionIndex] = createSignal(0);
  const battle = demoBattle;
  const selectedAction = createMemo(() => battle.actions[selectedActionIndex()] ?? battle.actions[0]);
  const sceneState = createMemo(() => ({
    turn: battle.turn,
    playerHpRatio: hpRatio(battle.player.hp, battle.player.maxHp),
    opponentHpRatio: hpRatio(battle.opponent.hp, battle.opponent.maxHp),
    selectedActionIndex: selectedActionIndex(),
  }));

  return (
    <main class={styles["page"]} aria-label="Shamrock battle demo">
      <section class={styles["stage"]} aria-label="Battlefield">
        <ShamrockScene state={sceneState()} />
        <div class={styles["battleMeta"]}>
          <span>{battle.battleId}</span>
          <span>Turn {battle.turn}</span>
          <span>{battle.weather}</span>
        </div>
        <SidePanel side={battle.opponent} className={styles["opponentPanel"]} />
        <SidePanel side={battle.player} className={styles["playerPanel"]} />
      </section>

      <section class={styles["commandArea"]} aria-label="Battle commands">
        <div class={styles["dialogBox"]}>
          <h1>What will {battle.player.name} do?</h1>
          <p>
            {selectedAction()
              ? `${selectedAction()?.label} is ready. ${actionSummary(selectedAction())}`
              : "Choose a move."}
          </p>
          <ul class={styles["logList"]} aria-label="Recent battle log">
            <For each={battle.log}>{(line) => <li>{line.text}</li>}</For>
          </ul>
        </div>
        <div class={styles["actionPanel"]} aria-label="Move menu">
          <For each={battle.actions}>
            {(action, index) => (
              <ActionButton
                action={action}
                selected={selectedActionIndex() === index()}
                onSelect={() => setSelectedActionIndex(index())}
              />
            )}
          </For>
        </div>
      </section>
    </main>
  );
}

function SidePanel(props: { side: BattleSideView; className: string | undefined }) {
  const ratio = () => hpRatio(props.side.hp, props.side.maxHp);
  return (
    <aside
      class={`${styles["sidePanel"] ?? ""} ${props.className ?? ""}`}
      aria-label={`${props.side.label} status`}
    >
      <div class={styles["sideHeader"]}>
        <span class={styles["nameLine"]}>{props.side.name}</span>
        <span class={styles["level"]}>Lv {props.side.level}</span>
      </div>
      <div class={styles["hpRow"]}>
        <span class={styles["hpLabel"]}>HP</span>
        <div class={styles["hpTrack"]} aria-label={`${props.side.name} HP`}>
          <span
            class={styles["hpFill"]}
            data-warning={ratio() < 0.55 && ratio() >= 0.25}
            data-danger={ratio() < 0.25}
            style={{ width: `${Math.round(ratio() * 100)}%` }}
          />
        </div>
      </div>
      <div class={styles["sideFooter"]}>
        <span>
          {props.side.hp}/{props.side.maxHp}
        </span>
        <span>{typeLine(props.side.primaryType, props.side.secondaryType)}</span>
        <span>
          {props.side.status} {props.side.partyAlive}/{props.side.partySize}
        </span>
      </div>
    </aside>
  );
}

function ActionButton(props: {
  action: BattleActionView;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      class={styles["actionButton"]}
      data-selected={props.selected}
      onClick={props.onSelect}
    >
      <span class={styles["actionName"]}>{props.action.label}</span>
      <span class={styles["actionMeta"]}>
        <span class={styles["typeBadge"]} data-type={props.action.element}>
          {props.action.element}
        </span>
        <span>{props.action.power === null ? "Status" : `Pow ${props.action.power}`}</span>
        <span>{props.action.pp}</span>
      </span>
    </button>
  );
}

function typeLine(primary: ElementType, secondary: ElementType | null): string {
  return secondary === null ? primary : `${primary}/${secondary}`;
}

function actionSummary(action: BattleActionView | undefined): string {
  if (!action) return "";
  if (action.power === null) return `${action.element} status move. PP ${action.pp}.`;
  return `${action.element} move, power ${action.power}. PP ${action.pp}.`;
}
