import Library from "./views/Library";
import Panel from "./views/Panel";
import Region from "./views/Region";
import Settings from "./views/Settings";

type View = "panel" | "region" | "settings" | "library";

// Each window loads index.html with a fixed hash (#/panel, #/region, #/settings,
// #/library) and keeps it for its whole lifetime -- windows are created when opened and
// destroyed when closed (docs/impl/ipc.md > Windows), so there is no in-page navigation
// between views.
function currentView(): View {
  const path = window.location.hash.replace(/^#/, "").split("?")[0];
  if (path === "/region") return "region";
  if (path === "/settings") return "settings";
  if (path === "/library") return "library";
  return "panel";
}

export default function App() {
  switch (currentView()) {
    case "region":
      return <Region />;
    case "settings":
      return <Settings />;
    case "library":
      return <Library />;
    default:
      return <Panel />;
  }
}
