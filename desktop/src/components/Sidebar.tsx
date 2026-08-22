import { useRef } from "react";

import { navigation, type NavigationName } from "../data/navigation";

interface SidebarProps {
  selected: NavigationName;
  onSelect: (name: NavigationName) => void;
}

export function Sidebar({ selected, onSelect }: SidebarProps) {
  const navRef = useRef<HTMLElement>(null);

  function moveFocus(current: HTMLButtonElement, direction: number) {
    const buttons = Array.from(navRef.current?.querySelectorAll<HTMLButtonElement>("button") ?? []);
    const index = buttons.indexOf(current);
    const target = buttons[(index + direction + buttons.length) % buttons.length];
    target?.focus();
  }

  return (
    <aside className="sidebar" aria-label="NEMESIS surfaces">
      <a className="skip-link" href="#workspace-content">Skip to mission workspace</a>
      <div className="identity-block">
        <div className="nemesis-mark" aria-hidden="true">N</div>
        <div>
          <h1>NEMESIS</h1>
          <p>The Provable Agent Operating System</p>
        </div>
      </div>

      <nav className="primary-nav" aria-label="Primary" ref={navRef}>
        {navigation.map(([name, code]) => (
          <button
            type="button"
            className={name === selected ? "nav-item is-selected" : "nav-item"}
            aria-current={name === selected ? "page" : undefined}
            onClick={() => onSelect(name)}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                moveFocus(event.currentTarget, 1);
              } else if (event.key === "ArrowUp") {
                event.preventDefault();
                moveFocus(event.currentTarget, -1);
              } else if (event.key === "Home") {
                event.preventDefault();
                navRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
              } else if (event.key === "End") {
                event.preventDefault();
                const buttons = navRef.current?.querySelectorAll<HTMLButtonElement>("button");
                buttons?.item(buttons.length - 1).focus();
              }
            }}
            key={name}
          >
            <span className="nav-code" aria-hidden="true">{code}</span>
            <span>{name}</span>
          </button>
        ))}
      </nav>

      <div className="doctrine-block">
        <span className="signal-line" aria-hidden="true" />
        <p>Intelligence proposes. NEMESIS governs. Evidence decides.</p>
      </div>
    </aside>
  );
}
