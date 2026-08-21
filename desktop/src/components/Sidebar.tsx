import { navigation, type NavigationName } from "../data/navigation";

interface SidebarProps {
  selected: NavigationName;
  onSelect: (name: NavigationName) => void;
}

export function Sidebar({ selected, onSelect }: SidebarProps) {
  return (
    <aside className="sidebar" aria-label="NEMESIS surfaces">
      <div className="identity-block">
        <div className="nemesis-mark" aria-hidden="true">
          N
        </div>
        <div>
          <h1>NEMESIS</h1>
          <p>The Provable Agent Operating System</p>
        </div>
      </div>

      <nav className="primary-nav" aria-label="Primary">
        {navigation.map(([name, code]) => (
          <button
            type="button"
            className={name === selected ? "nav-item is-selected" : "nav-item"}
            aria-current={name === selected ? "page" : undefined}
            onClick={() => onSelect(name)}
            key={name}
          >
            <span className="nav-code" aria-hidden="true">
              {code}
            </span>
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
