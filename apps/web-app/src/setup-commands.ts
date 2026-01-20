import {
  LayoutGrid,
  Plus,
  Search,
  Settings,
  User,
  Moon,
  RefreshCw,
  Download,
  Upload,
  Trash2,
  Filter,
  SortAsc,
  Key,
} from "lucide";
import type { CommandItem, CommandPalette } from "./components/command-palette";

const commands: CommandItem[] = [
  // Navigation
  {
    id: "nav-board",
    label: "Go to Board",
    description: "View kanban board",
    category: "Navigation",
    icon: LayoutGrid,
    shortcut: ["G", "B"],
    action: () => {
      document.querySelector("app-header")?.setAttribute("active-nav", "board");
    },
  },

  {
    id: "nav-credentials",
    label: "Go to Credentials",
    description: "Manage API keys and secrets",
    category: "Navigation",
    icon: Key,
    shortcut: ["G", "C"],
    action: () => {
      document.querySelector("app-header")?.setAttribute("active-nav", "credentials");
    },
  },

  // Actions
  {
    id: "create-task",
    label: "Create New Task",
    description: "Add a new task to the board",
    category: "Actions",
    icon: Plus,
    shortcut: ["C"],
    action: () => {
      console.log("Create task action triggered");
      alert("Create Task dialog would open here");
    },
  },
  {
    id: "search-tasks",
    label: "Search Tasks",
    description: "Find tasks by name or description",
    category: "Actions",
    icon: Search,
    shortcut: ["/"],
    action: () => {
      const searchInput = document.querySelector<HTMLInputElement>(".header-search input");
      searchInput?.focus();
    },
  },
  {
    id: "filter-tasks",
    label: "Filter Tasks",
    description: "Apply filters to tasks",
    category: "Actions",
    icon: Filter,
    action: () => {
      console.log("Filter tasks action triggered");
      alert("Filter panel would open here");
    },
  },
  {
    id: "sort-tasks",
    label: "Sort Tasks",
    description: "Change task sort order",
    category: "Actions",
    icon: SortAsc,
    action: () => {
      console.log("Sort tasks action triggered");
      alert("Sort options would appear here");
    },
  },
  {
    id: "refresh",
    label: "Refresh Data",
    description: "Reload all data from server",
    category: "Actions",
    icon: RefreshCw,
    shortcut: ["R"],
    action: () => {
      console.log("Refresh action triggered");
      window.location.reload();
    },
  },

  // Data
  {
    id: "export-data",
    label: "Export Data",
    description: "Download tasks as JSON or CSV",
    category: "Data",
    icon: Download,
    action: () => {
      console.log("Export data action triggered");
      alert("Export dialog would open here");
    },
  },
  {
    id: "import-data",
    label: "Import Data",
    description: "Upload tasks from file",
    category: "Data",
    icon: Upload,
    action: () => {
      console.log("Import data action triggered");
      alert("Import dialog would open here");
    },
  },
  {
    id: "clear-completed",
    label: "Clear Completed Tasks",
    description: "Remove all completed tasks",
    category: "Data",
    icon: Trash2,
    action: () => {
      console.log("Clear completed action triggered");
      if (confirm("Are you sure you want to clear all completed tasks?")) {
        alert("Completed tasks cleared");
      }
    },
  },

  // Settings
  {
    id: "settings",
    label: "Open Settings",
    description: "Configure application preferences",
    category: "Settings",
    icon: Settings,
    shortcut: [","],
    action: () => {
      console.log("Settings action triggered");
      alert("Settings panel would open here");
    },
  },
  {
    id: "profile",
    label: "View Profile",
    description: "Open your user profile",
    category: "Settings",
    icon: User,
    action: () => {
      console.log("Profile action triggered");
      alert("Profile panel would open here");
    },
  },
  {
    id: "toggle-theme",
    label: "Toggle Dark/Light Mode",
    description: "Switch between dark and light themes",
    category: "Settings",
    icon: Moon,
    action: () => {
      document.documentElement.classList.toggle("dark");
      console.log("Theme toggled");
    },
  },
];

// Initialize command palette when DOM is ready
const initCommandPalette = () => {
  const palette = document.querySelector<CommandPalette>("command-palette");
  if (palette) {
    palette.commands = commands;
    console.log("Command palette initialized with", commands.length, "commands");
  }
};

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initCommandPalette);
} else {
  initCommandPalette();
}
