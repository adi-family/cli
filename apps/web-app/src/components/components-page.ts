import { LitElement, html, css } from "lit";
import { customElement, state } from "lit/decorators.js";
import "./loading-indicators";
import "./buttons";
import "./inputs";
import { SoundController } from "./sounds";
import type { SoundType } from "./sounds";

type TabId = "loading" | "speceffects" | "fullpageeffects" | "buttons" | "inputs" | "sounds" | "feedback";

interface ComponentInfo {
  name: string;
  tag: string;
  description: string;
  props: { name: string; type: string; default?: string; description: string }[];
}

@customElement("components-page")
export class ComponentsPage extends LitElement {
  @state() private activeTab: TabId = "loading";
  @state() private selectedComponent: string | null = null;

  // Sound controller - listens for sound events in this component tree
  private soundController = new SoundController(this);

  private loadingComponents: ComponentInfo[] = [
    {
      name: "Loading Skeleton",
      tag: "loading-skeleton",
      description: "Shimmer placeholder effect for content loading states",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the skeleton" },
        { name: "variant", type: '"card" | "text" | "avatar"', default: '"card"', description: "Shape variant" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Ripple Effect",
      tag: "ripple-effect",
      description: "Expanding water ripple animation",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the ripple" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Morphing Blob",
      tag: "morphing-blob",
      description: "Organic SVG shape that continuously morphs",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the blob" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Matrix Rain",
      tag: "matrix-rain",
      description: "Falling characters effect inspired by The Matrix",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the canvas" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Gradient Sweep",
      tag: "gradient-sweep",
      description: "Circular progress with animated sweeping gradient",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the circle" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Wave Bar",
      tag: "wave-bar",
      description: "Audio visualizer-style bouncing bars",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the bars" },
        { name: "bars", type: "number", default: "5", description: "Number of bars" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
  ];

  private specEffectsComponents: ComponentInfo[] = [
    {
      name: "Particle Explosion",
      tag: "particle-explosion",
      description: "Canvas-based particles that burst and regenerate",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the canvas" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Confetti Burst",
      tag: "confetti-burst",
      description: "Colorful confetti explosion for congratulations",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the canvas" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Sparkle Burst",
      tag: "sparkle-burst",
      description: "Radiating star sparkles for attention and highlights",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the canvas" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Fireworks",
      tag: "fireworks-effect",
      description: "Rising rockets that explode into colorful sparks",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the canvas" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
  ];

  private fullPageEffectsComponents: ComponentInfo[] = [
    {
      name: "Fullpage Confetti",
      tag: "fullpage-confetti",
      description: "Full-screen confetti overlay for celebrations and achievements",
      props: [
        { name: "intensity", type: "number", default: "80", description: "Number of confetti pieces per burst" },
        { name: "duration", type: "number", default: "3000", description: "Effect duration in ms" },
      ],
    },
    {
      name: "Fullpage Fireworks",
      tag: "fullpage-fireworks",
      description: "Full-screen fireworks overlay with rockets and explosions",
      props: [
        { name: "rockets", type: "number", default: "5", description: "Number of rockets to launch" },
      ],
    },
    {
      name: "Fullpage Aurora",
      tag: "fullpage-aurora",
      description: "Northern lights overlay effect for ambient celebrations",
      props: [
        { name: "duration", type: "number", default: "4000", description: "Effect duration in ms" },
      ],
    },
    {
      name: "Fullpage Starfield",
      tag: "fullpage-starfield",
      description: "Warp speed starfield overlay for level-ups and achievements",
      props: [
        { name: "duration", type: "number", default: "3000", description: "Effect duration in ms" },
        { name: "speed", type: "number", default: "15", description: "Star travel speed" },
      ],
    },
  ];

  private buttonComponents: ComponentInfo[] = [
    {
      name: "Primary Button",
      tag: "primary-button",
      description: "Main call-to-action button with gradient background",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the button" },
        { name: "label", type: "string", default: '"Button"', description: "Button text" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the button" },
        { name: "loading", type: "boolean", default: "false", description: "Show loading spinner" },
      ],
    },
    {
      name: "Secondary Button",
      tag: "secondary-button",
      description: "Outlined button for secondary actions",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the button" },
        { name: "label", type: "string", default: '"Button"', description: "Button text" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the button" },
        { name: "loading", type: "boolean", default: "false", description: "Show loading spinner" },
      ],
    },
    {
      name: "Ghost Button",
      tag: "ghost-button",
      description: "Subtle button with no background",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the button" },
        { name: "label", type: "string", default: '"Button"', description: "Button text" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the button" },
        { name: "loading", type: "boolean", default: "false", description: "Show loading spinner" },
      ],
    },
    {
      name: "Danger Button",
      tag: "danger-button",
      description: "Red button for destructive actions",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the button" },
        { name: "label", type: "string", default: '"Delete"', description: "Button text" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the button" },
        { name: "loading", type: "boolean", default: "false", description: "Show loading spinner" },
      ],
    },
    {
      name: "Icon Button",
      tag: "icon-button",
      description: "Square button for icon-only actions",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the button" },
        { name: "icon", type: "string", description: "Icon content (emoji or slot)" },
        { name: "label", type: "string", description: "Accessible label / tooltip" },
        { name: "variant", type: '"default" | "primary" | "danger"', default: '"default"', description: "Visual style" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the button" },
      ],
    },
    {
      name: "Button Group",
      tag: "button-group",
      description: "Segmented control for selecting one option from a group",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the buttons" },
        { name: "value", type: "string", default: '""', description: "Selected value" },
        { name: "options", type: "ButtonGroupOption[]", description: "Array of options {value, label, disabled?}" },
        { name: "variant", type: '"default" | "primary"', default: '"default"', description: "Visual style" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable all buttons" },
      ],
    },
  ];

  private soundComponents: ComponentInfo[] = [
    {
      name: "UI Click",
      tag: "ui-click",
      description: "Clean tap sound for button feedback",
      props: [
        { name: "volume", type: "number", default: "0.3", description: "Volume level (0-1)" },
      ],
    },
    {
      name: "Success Chime",
      tag: "success-chime",
      description: "Pleasant two-note ascending chime",
      props: [
        { name: "volume", type: "number", default: "0.25", description: "Volume level (0-1)" },
      ],
    },
    {
      name: "Error Tone",
      tag: "error-tone",
      description: "Short descending tone for errors",
      props: [
        { name: "volume", type: "number", default: "0.2", description: "Volume level (0-1)" },
      ],
    },
    {
      name: "Notification Ding",
      tag: "notification-ding",
      description: "Clear bell-like notification tone",
      props: [
        { name: "volume", type: "number", default: "0.25", description: "Volume level (0-1)" },
      ],
    },
    {
      name: "Whoosh",
      tag: "whoosh",
      description: "Smooth sweep for page transitions",
      props: [
        { name: "volume", type: "number", default: "0.15", description: "Volume level (0-1)" },
      ],
    },
    // File-based sounds from public/sounds/
    {
      name: "Confetti",
      tag: "confetti",
      description: "Celebration pop sound from audio file",
      props: [
        { name: "volume", type: "number", default: "0.5", description: "Volume level (0-1)" },
      ],
    },
    {
      name: "Error (File)",
      tag: "error-file",
      description: "Error tone from audio file",
      props: [
        { name: "volume", type: "number", default: "0.5", description: "Volume level (0-1)" },
      ],
    },
    {
      name: "Firework",
      tag: "firework",
      description: "Explosive celebration sound",
      props: [
        { name: "volume", type: "number", default: "0.5", description: "Volume level (0-1)" },
      ],
    },
    {
      name: "Magic",
      tag: "magic",
      description: "Sparkle/enchantment sound effect",
      props: [
        { name: "volume", type: "number", default: "0.5", description: "Volume level (0-1)" },
      ],
    },
    {
      name: "Success (File)",
      tag: "success-file",
      description: "Achievement unlock sound",
      props: [
        { name: "volume", type: "number", default: "0.5", description: "Volume level (0-1)" },
      ],
    },
    {
      name: "Warning",
      tag: "warning",
      description: "Alert/caution sound from audio file",
      props: [
        { name: "volume", type: "number", default: "0.5", description: "Volume level (0-1)" },
      ],
    },
  ];

  private inputComponents: ComponentInfo[] = [
    {
      name: "Text Input",
      tag: "text-input",
      description: "Standard text input field with label and validation",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the input" },
        { name: "value", type: "string", default: '""', description: "Input value" },
        { name: "placeholder", type: "string", default: '"Enter text..."', description: "Placeholder text" },
        { name: "label", type: "string", description: "Label above input" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the input" },
        { name: "error", type: "boolean", default: "false", description: "Show error state" },
        { name: "errorMessage", type: "string", description: "Error message to display" },
      ],
    },
    {
      name: "Search Input",
      tag: "search-input",
      description: "Search field with icon and clear button",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the input" },
        { name: "value", type: "string", default: '""', description: "Search value" },
        { name: "placeholder", type: "string", default: '"Search..."', description: "Placeholder text" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the input" },
        { name: "loading", type: "boolean", default: "false", description: "Show loading spinner" },
      ],
    },
    {
      name: "Textarea",
      tag: "textarea-input",
      description: "Multi-line text input for longer content",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the textarea" },
        { name: "value", type: "string", default: '""', description: "Textarea value" },
        { name: "placeholder", type: "string", default: '"Enter text..."', description: "Placeholder text" },
        { name: "label", type: "string", description: "Label above textarea" },
        { name: "rows", type: "number", default: "4", description: "Number of visible rows" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the textarea" },
        { name: "error", type: "boolean", default: "false", description: "Show error state" },
      ],
    },
    {
      name: "Select",
      tag: "select-input",
      description: "Dropdown select with custom styling",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the select" },
        { name: "value", type: "string", default: '""', description: "Selected value" },
        { name: "placeholder", type: "string", default: '"Select option..."', description: "Placeholder text" },
        { name: "label", type: "string", description: "Label above select" },
        { name: "options", type: "SelectOption[]", description: "Array of options" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the select" },
      ],
    },
    {
      name: "Checkbox",
      tag: "checkbox-input",
      description: "Checkbox with optional label",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the checkbox" },
        { name: "checked", type: "boolean", default: "false", description: "Checked state" },
        { name: "label", type: "string", description: "Label next to checkbox" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the checkbox" },
      ],
    },
    {
      name: "Toggle",
      tag: "toggle-input",
      description: "Toggle switch for on/off states",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the toggle" },
        { name: "checked", type: "boolean", default: "false", description: "Checked state" },
        { name: "label", type: "string", description: "Label next to toggle" },
        { name: "disabled", type: "boolean", default: "false", description: "Disable the toggle" },
      ],
    },
  ];

  static styles = css`
    :host {
      display: block;
      min-height: calc(100vh - 4rem);
      background: #0d0a14;
      color: #d1d5db;
      font-family: 'Inter', system-ui, sans-serif;
      overflow: auto;
    }

    .page {
      max-width: 1400px;
      margin: 0 auto;
      padding: 2rem;
      padding-bottom: 4rem;
    }

    .header {
      margin-bottom: 2rem;
    }

    .header h1 {
      font-size: 2rem;
      font-weight: 700;
      color: white;
      margin: 0 0 0.5rem;
    }

    .header p {
      color: #9ca3af;
      margin: 0;
    }

    .tabs {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 2rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
      padding-bottom: 1rem;
    }

    .tab {
      padding: 0.625rem 1rem;
      border: none;
      background: transparent;
      color: #9ca3af;
      font-size: 0.875rem;
      font-weight: 500;
      cursor: pointer;
      border-radius: 0.5rem;
      transition: all 0.2s;
      font-family: inherit;
    }

    .tab:hover {
      color: white;
      background: rgba(139, 92, 246, 0.1);
    }

    .tab.active {
      color: white;
      background: rgba(139, 92, 246, 0.2);
    }

    .tab.disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }

    .content {
      display: grid;
      grid-template-columns: 260px 1fr;
      gap: 2rem;
    }

    @media (max-width: 900px) {
      .content {
        grid-template-columns: 1fr;
      }
    }

    .sidebar {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .sidebar-item {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 0.75rem 1rem;
      border: none;
      background: transparent;
      color: #9ca3af;
      font-size: 0.875rem;
      text-align: left;
      cursor: pointer;
      border-radius: 0.5rem;
      transition: all 0.2s;
      font-family: inherit;
    }

    .sidebar-item:hover {
      color: white;
      background: rgba(255, 255, 255, 0.05);
    }

    .sidebar-item.active {
      color: white;
      background: rgba(139, 92, 246, 0.15);
      border-left: 2px solid #8b5cf6;
    }

    .sidebar-item-icon {
      width: 32px;
      height: 32px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: rgba(139, 92, 246, 0.1);
      border-radius: 0.375rem;
      flex-shrink: 0;
      overflow: hidden;
    }

    .main {
      background: #13101c;
      border-radius: 1rem;
      border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .main-header {
      padding: 1.5rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .main-header h2 {
      font-size: 1.25rem;
      font-weight: 600;
      color: white;
      margin: 0 0 0.25rem;
    }

    .main-header p {
      color: #6b7280;
      font-size: 0.875rem;
      margin: 0;
    }

    .preview {
      padding: 2rem;
      background: rgba(0, 0, 0, 0.2);
      display: flex;
      justify-content: center;
      align-items: center;
      min-height: 200px;
    }

    .preview-sizes {
      display: flex;
      gap: 3rem;
      align-items: flex-end;
    }

    .preview-item {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.75rem;
    }

    .preview-item span {
      font-size: 0.75rem;
      color: #6b7280;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .props {
      padding: 1.5rem;
    }

    .props h3 {
      font-size: 0.875rem;
      font-weight: 600;
      color: white;
      margin: 0 0 1rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .props-table {
      width: 100%;
      border-collapse: collapse;
      font-size: 0.875rem;
    }

    .props-table th {
      text-align: left;
      padding: 0.75rem;
      color: #9ca3af;
      font-weight: 500;
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .props-table td {
      padding: 0.75rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    }

    .props-table tr:last-child td {
      border-bottom: none;
    }

    .prop-name {
      color: #a78bfa;
      font-family: 'JetBrains Mono', monospace;
    }

    .prop-type {
      color: #6b7280;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.75rem;
    }

    .prop-default {
      color: #4ade80;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.75rem;
    }

    .code {
      padding: 1.5rem;
      border-top: 1px solid rgba(255, 255, 255, 0.1);
    }

    .code h3 {
      font-size: 0.875rem;
      font-weight: 600;
      color: white;
      margin: 0 0 1rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .code-block {
      background: #0d0a14;
      border-radius: 0.5rem;
      padding: 1rem;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.8125rem;
      color: #a78bfa;
      overflow-x: auto;
    }

    .code-tag { color: #f472b6; }
    .code-attr { color: #67e8f9; }
    .code-value { color: #4ade80; }

    .empty {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 4rem;
      color: #6b7280;
      text-align: center;
    }

    .empty-icon {
      font-size: 3rem;
      margin-bottom: 1rem;
      opacity: 0.5;
    }

    /* Sound trigger buttons */
    .sound-btn {
      display: inline-flex;
      flex-direction: column;
      align-items: center;
      gap: 0.5rem;
      padding: 1rem;
      border-radius: 0.75rem;
      border: none;
      cursor: pointer;
      transition: all 0.2s;
      font-family: inherit;
    }

    .sound-btn:hover {
      transform: scale(1.02);
    }

    .sound-btn:active {
      transform: scale(0.98);
    }

    .sound-btn .icon {
      font-size: 2rem;
      line-height: 1;
    }

    .sound-btn .label {
      font-weight: 500;
      font-size: 0.875rem;
    }

    .sound-btn.ui-click {
      background: rgba(99, 102, 241, 0.1);
      border: 1px solid rgba(99, 102, 241, 0.3);
      color: #818cf8;
    }
    .sound-btn.ui-click:hover {
      background: rgba(99, 102, 241, 0.15);
    }

    .sound-btn.success-chime, .sound-btn.success-file {
      background: rgba(34, 197, 94, 0.1);
      border: 1px solid rgba(34, 197, 94, 0.3);
      color: #4ade80;
    }
    .sound-btn.success-chime:hover, .sound-btn.success-file:hover {
      background: rgba(34, 197, 94, 0.15);
    }

    .sound-btn.error-tone, .sound-btn.error-file {
      background: rgba(239, 68, 68, 0.1);
      border: 1px solid rgba(239, 68, 68, 0.3);
      color: #f87171;
    }
    .sound-btn.error-tone:hover, .sound-btn.error-file:hover {
      background: rgba(239, 68, 68, 0.15);
    }

    .sound-btn.notification-ding {
      background: rgba(251, 191, 36, 0.1);
      border: 1px solid rgba(251, 191, 36, 0.3);
      color: #fbbf24;
    }
    .sound-btn.notification-ding:hover {
      background: rgba(251, 191, 36, 0.15);
    }

    .sound-btn.whoosh {
      background: rgba(139, 92, 246, 0.1);
      border: 1px solid rgba(139, 92, 246, 0.3);
      color: #a78bfa;
    }
    .sound-btn.whoosh:hover {
      background: rgba(139, 92, 246, 0.15);
    }

    .sound-btn.confetti {
      background: rgba(236, 72, 153, 0.1);
      border: 1px solid rgba(236, 72, 153, 0.3);
      color: #f472b6;
    }
    .sound-btn.confetti:hover {
      background: rgba(236, 72, 153, 0.15);
    }

    .sound-btn.firework {
      background: rgba(249, 115, 22, 0.1);
      border: 1px solid rgba(249, 115, 22, 0.3);
      color: #fb923c;
    }
    .sound-btn.firework:hover {
      background: rgba(249, 115, 22, 0.15);
    }

    .sound-btn.magic {
      background: rgba(168, 85, 247, 0.1);
      border: 1px solid rgba(168, 85, 247, 0.3);
      color: #c084fc;
    }
    .sound-btn.magic:hover {
      background: rgba(168, 85, 247, 0.15);
    }

    .sound-btn.warning {
      background: rgba(234, 179, 8, 0.1);
      border: 1px solid rgba(234, 179, 8, 0.3);
      color: #facc15;
    }
    .sound-btn.warning:hover {
      background: rgba(234, 179, 8, 0.15);
    }
  `;

  private renderSidebarIcon(tag: string) {
    switch (tag) {
      case "loading-skeleton":
        return html`<loading-skeleton size="sm" variant="text"></loading-skeleton>`;
      case "ripple-effect":
        return html`<ripple-effect size="sm"></ripple-effect>`;
      case "morphing-blob":
        return html`<morphing-blob size="sm"></morphing-blob>`;
      case "matrix-rain":
        return html`<matrix-rain size="sm"></matrix-rain>`;
      case "particle-explosion":
        return html`<particle-explosion size="sm"></particle-explosion>`;
      case "gradient-sweep":
        return html`<gradient-sweep size="sm"></gradient-sweep>`;
      case "wave-bar":
        return html`<wave-bar size="sm" bars="3"></wave-bar>`;
      case "confetti-burst":
        return html`<confetti-burst size="sm"></confetti-burst>`;
      case "sparkle-burst":
        return html`<sparkle-burst size="sm"></sparkle-burst>`;
      case "fireworks-effect":
        return html`<fireworks-effect size="sm"></fireworks-effect>`;
      // Full-page effects (show mini preview)
      case "fullpage-confetti":
        return html`<confetti-burst size="sm"></confetti-burst>`;
      case "fullpage-fireworks":
        return html`<fireworks-effect size="sm"></fireworks-effect>`;
      case "fullpage-aurora":
        return html`<div style="width: 32px; height: 32px; background: linear-gradient(135deg, #a78bfa, #67e8f9, #4ade80); border-radius: 4px; opacity: 0.8;"></div>`;
      case "fullpage-starfield":
        return html`<div style="width: 32px; height: 32px; background: radial-gradient(circle, #1a1030 0%, #0a0815 100%); border-radius: 4px; position: relative;">
          <div style="position: absolute; width: 2px; height: 2px; background: white; top: 8px; left: 10px; border-radius: 50%;"></div>
          <div style="position: absolute; width: 1px; height: 1px; background: white; top: 15px; left: 20px; border-radius: 50%;"></div>
          <div style="position: absolute; width: 2px; height: 2px; background: #a78bfa; top: 20px; left: 8px; border-radius: 50%;"></div>
        </div>`;
      // Buttons
      case "primary-button":
        return html`<primary-button size="sm" label="Btn"></primary-button>`;
      case "secondary-button":
        return html`<secondary-button size="sm" label="Btn"></secondary-button>`;
      case "ghost-button":
        return html`<ghost-button size="sm" label="Btn"></ghost-button>`;
      case "danger-button":
        return html`<danger-button size="sm" label="Del"></danger-button>`;
      case "icon-button":
        return html`<icon-button size="sm" icon="+" variant="primary"></icon-button>`;
      case "button-group":
        return html`<button-group size="sm" value="a" .options=${[{ value: "a", label: "A" }, { value: "b", label: "B" }]}></button-group>`;
      // Inputs
      case "text-input":
        return html`<text-input size="sm" placeholder="Text" style="width: 60px;"></text-input>`;
      case "search-input":
        return html`<search-input size="sm" placeholder="..." style="width: 60px;"></search-input>`;
      case "textarea-input":
        return html`<textarea-input size="sm" placeholder="..." rows="1" style="width: 60px;"></textarea-input>`;
      case "select-input":
        return html`<select-input size="sm" placeholder="..." style="width: 60px;"></select-input>`;
      case "checkbox-input":
        return html`<checkbox-input size="sm"></checkbox-input>`;
      case "toggle-input":
        return html`<toggle-input size="sm"></toggle-input>`;
      // Sound types (event-based)
      case "ui-click":
        return html`<div style="width: 32px; height: 32px; background: rgba(99, 102, 241, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 10px; color: #818cf8;">tap</div>`;
      case "success-chime":
        return html`<div style="width: 32px; height: 32px; background: rgba(34, 197, 94, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 10px; color: #4ade80;">ok</div>`;
      case "error-tone":
        return html`<div style="width: 32px; height: 32px; background: rgba(239, 68, 68, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 10px; color: #f87171;">err</div>`;
      case "notification-ding":
        return html`<div style="width: 32px; height: 32px; background: rgba(251, 191, 36, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 14px; color: #fbbf24;">!</div>`;
      case "whoosh":
        return html`<div style="width: 32px; height: 32px; background: rgba(139, 92, 246, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 10px; color: #a78bfa;">~></div>`;
      // File-based sounds
      case "confetti":
        return html`<div style="width: 32px; height: 32px; background: rgba(236, 72, 153, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 10px; color: #f472b6;">pop</div>`;
      case "error-file":
        return html`<div style="width: 32px; height: 32px; background: rgba(239, 68, 68, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 10px; color: #f87171;">err</div>`;
      case "firework":
        return html`<div style="width: 32px; height: 32px; background: rgba(249, 115, 22, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 9px; color: #fb923c;">boom</div>`;
      case "magic":
        return html`<div style="width: 32px; height: 32px; background: rgba(168, 85, 247, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 14px; color: #c084fc;">*</div>`;
      case "success-file":
        return html`<div style="width: 32px; height: 32px; background: rgba(34, 197, 94, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 10px; color: #4ade80;">ok</div>`;
      case "warning":
        return html`<div style="width: 32px; height: 32px; background: rgba(234, 179, 8, 0.2); border-radius: 4px; display: flex; align-items: center; justify-content: center; font-size: 14px; color: #facc15;">!</div>`;
      default:
        return html``;
    }
  }

  private getComponentsForTab(): ComponentInfo[] {
    switch (this.activeTab) {
      case "loading":
        return this.loadingComponents;
      case "speceffects":
        return this.specEffectsComponents;
      case "fullpageeffects":
        return this.fullPageEffectsComponents;
      case "buttons":
        return this.buttonComponents;
      case "inputs":
        return this.inputComponents;
      case "sounds":
        return this.soundComponents;
      default:
        return [];
    }
  }

  private renderSidebar() {
    const components = this.getComponentsForTab();

    return html`
      <div class="sidebar">
        ${components.map(
          (c) => html`
            <button
              class="sidebar-item ${this.selectedComponent === c.tag ? "active" : ""}"
              @click=${() => (this.selectedComponent = c.tag)}
            >
              <div class="sidebar-item-icon">
                ${this.renderSidebarIcon(c.tag)}
              </div>
              ${c.name}
            </button>
          `
        )}
      </div>
    `;
  }

  private getSoundButtonInfo(sound: SoundType): { icon: string; label: string } {
    switch (sound) {
      case "ui-click": return { icon: "tap", label: "Click" };
      case "success-chime": return { icon: "ok", label: "Success" };
      case "error-tone": return { icon: "err", label: "Error" };
      case "notification-ding": return { icon: "!", label: "Ding" };
      case "whoosh": return { icon: "~>", label: "Whoosh" };
      case "confetti": return { icon: "pop", label: "Confetti" };
      case "error-file": return { icon: "err", label: "Error" };
      case "firework": return { icon: "boom", label: "Firework" };
      case "magic": return { icon: "*", label: "Magic" };
      case "success-file": return { icon: "ok", label: "Success" };
      case "warning": return { icon: "!", label: "Warning" };
      default: return { icon: "?", label: "Sound" };
    }
  }

  private renderSoundButton(sound: SoundType) {
    const { icon, label } = this.getSoundButtonInfo(sound);
    return html`
      <button 
        class="sound-btn ${sound}" 
        @click=${() => this.soundController.play(sound)}
      >
        <span class="icon">${icon}</span>
        <span class="label">${label}</span>
      </button>
    `;
  }

  private renderComponentPreview(tag: string) {
    switch (tag) {
      case "loading-skeleton":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <loading-skeleton size="sm" variant="card"></loading-skeleton>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <loading-skeleton size="md" variant="card"></loading-skeleton>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <loading-skeleton size="lg" variant="card"></loading-skeleton>
              <span>Large</span>
            </div>
          </div>
        `;
      case "ripple-effect":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <ripple-effect size="sm"></ripple-effect>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <ripple-effect size="md"></ripple-effect>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <ripple-effect size="lg"></ripple-effect>
              <span>Large</span>
            </div>
          </div>
        `;
      case "morphing-blob":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <morphing-blob size="sm"></morphing-blob>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <morphing-blob size="md"></morphing-blob>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <morphing-blob size="lg"></morphing-blob>
              <span>Large</span>
            </div>
          </div>
        `;
      case "matrix-rain":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <matrix-rain size="sm"></matrix-rain>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <matrix-rain size="md"></matrix-rain>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <matrix-rain size="lg"></matrix-rain>
              <span>Large</span>
            </div>
          </div>
        `;
      case "particle-explosion":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <particle-explosion size="sm"></particle-explosion>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <particle-explosion size="md"></particle-explosion>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <particle-explosion size="lg"></particle-explosion>
              <span>Large</span>
            </div>
          </div>
        `;
      case "gradient-sweep":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <gradient-sweep size="sm"></gradient-sweep>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <gradient-sweep size="md"></gradient-sweep>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <gradient-sweep size="lg"></gradient-sweep>
              <span>Large</span>
            </div>
          </div>
        `;
      case "wave-bar":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <wave-bar size="sm"></wave-bar>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <wave-bar size="md" bars="7"></wave-bar>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <wave-bar size="lg" bars="9"></wave-bar>
              <span>Large</span>
            </div>
          </div>
        `;
      case "confetti-burst":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <confetti-burst size="sm"></confetti-burst>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <confetti-burst size="md"></confetti-burst>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <confetti-burst size="lg"></confetti-burst>
              <span>Large</span>
            </div>
          </div>
        `;
      case "sparkle-burst":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <sparkle-burst size="sm"></sparkle-burst>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <sparkle-burst size="md"></sparkle-burst>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <sparkle-burst size="lg"></sparkle-burst>
              <span>Large</span>
            </div>
          </div>
        `;
      case "fireworks-effect":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <fireworks-effect size="sm"></fireworks-effect>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <fireworks-effect size="md"></fireworks-effect>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <fireworks-effect size="lg"></fireworks-effect>
              <span>Large</span>
            </div>
          </div>
        `;
      // Full-page effects - show trigger button
      case "fullpage-confetti":
        return html`
          <div style="display: flex; flex-direction: column; align-items: center; gap: 1rem;">
            <button
              @click=${() => (this.shadowRoot?.querySelector("fullpage-confetti") as any)?.trigger()}
              style="padding: 1rem 2rem; background: linear-gradient(135deg, #8b5cf6, #ec4899); border: none; border-radius: 0.5rem; color: white; font-weight: 600; cursor: pointer; font-size: 1rem;"
            >
              Trigger Confetti
            </button>
            <span style="color: #6b7280; font-size: 0.875rem;">Click to see full-page effect</span>
            <fullpage-confetti></fullpage-confetti>
          </div>
        `;
      case "fullpage-fireworks":
        return html`
          <div style="display: flex; flex-direction: column; align-items: center; gap: 1rem;">
            <button
              @click=${() => (this.shadowRoot?.querySelector("fullpage-fireworks") as any)?.trigger()}
              style="padding: 1rem 2rem; background: linear-gradient(135deg, #f97316, #fbbf24); border: none; border-radius: 0.5rem; color: white; font-weight: 600; cursor: pointer; font-size: 1rem;"
            >
              Launch Fireworks
            </button>
            <span style="color: #6b7280; font-size: 0.875rem;">Click to see full-page effect</span>
            <fullpage-fireworks></fullpage-fireworks>
          </div>
        `;
      case "fullpage-aurora":
        return html`
          <div style="display: flex; flex-direction: column; align-items: center; gap: 1rem;">
            <button
              @click=${() => (this.shadowRoot?.querySelector("fullpage-aurora") as any)?.trigger()}
              style="padding: 1rem 2rem; background: linear-gradient(135deg, #4ade80, #67e8f9); border: none; border-radius: 0.5rem; color: #0d0a14; font-weight: 600; cursor: pointer; font-size: 1rem;"
            >
              Trigger Aurora
            </button>
            <span style="color: #6b7280; font-size: 0.875rem;">Click to see full-page effect</span>
            <fullpage-aurora></fullpage-aurora>
          </div>
        `;
      case "fullpage-starfield":
        return html`
          <div style="display: flex; flex-direction: column; align-items: center; gap: 1rem;">
            <button
              @click=${() => (this.shadowRoot?.querySelector("fullpage-starfield") as any)?.trigger()}
              style="padding: 1rem 2rem; background: linear-gradient(135deg, #1a1030, #3b0764); border: 1px solid #8b5cf6; border-radius: 0.5rem; color: white; font-weight: 600; cursor: pointer; font-size: 1rem;"
            >
              Enter Warp Speed
            </button>
            <span style="color: #6b7280; font-size: 0.875rem;">Click to see full-page effect</span>
            <fullpage-starfield></fullpage-starfield>
          </div>
        `;
      // Button components
      case "primary-button":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <primary-button size="sm">Small</primary-button>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <primary-button size="md">Medium</primary-button>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <primary-button size="lg">Large</primary-button>
              <span>Large</span>
            </div>
            <div class="preview-item">
              <primary-button size="md" loading>Loading</primary-button>
              <span>Loading</span>
            </div>
            <div class="preview-item">
              <primary-button size="md" disabled>Disabled</primary-button>
              <span>Disabled</span>
            </div>
          </div>
        `;
      case "secondary-button":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <secondary-button size="sm">Small</secondary-button>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <secondary-button size="md">Medium</secondary-button>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <secondary-button size="lg">Large</secondary-button>
              <span>Large</span>
            </div>
            <div class="preview-item">
              <secondary-button size="md" loading>Loading</secondary-button>
              <span>Loading</span>
            </div>
            <div class="preview-item">
              <secondary-button size="md" disabled>Disabled</secondary-button>
              <span>Disabled</span>
            </div>
          </div>
        `;
      case "ghost-button":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <ghost-button size="sm">Small</ghost-button>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <ghost-button size="md">Medium</ghost-button>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <ghost-button size="lg">Large</ghost-button>
              <span>Large</span>
            </div>
            <div class="preview-item">
              <ghost-button size="md" loading>Loading</ghost-button>
              <span>Loading</span>
            </div>
            <div class="preview-item">
              <ghost-button size="md" disabled>Disabled</ghost-button>
              <span>Disabled</span>
            </div>
          </div>
        `;
      case "danger-button":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <danger-button size="sm">Delete</danger-button>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <danger-button size="md">Delete</danger-button>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <danger-button size="lg">Delete</danger-button>
              <span>Large</span>
            </div>
            <div class="preview-item">
              <danger-button size="md" loading>Deleting</danger-button>
              <span>Loading</span>
            </div>
            <div class="preview-item">
              <danger-button size="md" disabled>Delete</danger-button>
              <span>Disabled</span>
            </div>
          </div>
        `;
      case "icon-button":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <icon-button size="sm" icon="+"></icon-button>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <icon-button size="md" icon="+"></icon-button>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <icon-button size="lg" icon="+"></icon-button>
              <span>Large</span>
            </div>
            <div class="preview-item">
              <icon-button size="md" icon="+" variant="primary"></icon-button>
              <span>Primary</span>
            </div>
            <div class="preview-item">
              <icon-button size="md" icon="x" variant="danger"></icon-button>
              <span>Danger</span>
            </div>
          </div>
        `;
      case "button-group":
        return html`
          <div class="preview-sizes" style="flex-direction: column; gap: 1.5rem; align-items: flex-start;">
            <div class="preview-item" style="flex-direction: row; gap: 1rem;">
              <button-group 
                size="sm" 
                value="left"
                .options=${[{ value: "left", label: "Left" }, { value: "center", label: "Center" }, { value: "right", label: "Right" }]}
              ></button-group>
              <span style="color: #6b7280;">Small</span>
            </div>
            <div class="preview-item" style="flex-direction: row; gap: 1rem;">
              <button-group 
                size="md" 
                value="monthly"
                .options=${[{ value: "daily", label: "Daily" }, { value: "weekly", label: "Weekly" }, { value: "monthly", label: "Monthly" }]}
              ></button-group>
              <span style="color: #6b7280;">Medium</span>
            </div>
            <div class="preview-item" style="flex-direction: row; gap: 1rem;">
              <button-group 
                size="lg" 
                value="b"
                .options=${[{ value: "a", label: "Option A" }, { value: "b", label: "Option B" }, { value: "c", label: "Option C" }]}
              ></button-group>
              <span style="color: #6b7280;">Large</span>
            </div>
            <div class="preview-item" style="flex-direction: row; gap: 1rem;">
              <button-group 
                size="md" 
                value="grid"
                variant="primary"
                .options=${[{ value: "list", label: "List" }, { value: "grid", label: "Grid" }, { value: "table", label: "Table" }]}
              ></button-group>
              <span style="color: #6b7280;">Primary Variant</span>
            </div>
          </div>
        `;
      // Input components
      case "text-input":
        return html`
          <div class="preview-sizes" style="gap: 2rem;">
            <div class="preview-item">
              <text-input size="sm" placeholder="Small input"></text-input>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <text-input size="md" placeholder="Medium input" label="Label"></text-input>
              <span>Medium + Label</span>
            </div>
            <div class="preview-item">
              <text-input size="lg" placeholder="Large input"></text-input>
              <span>Large</span>
            </div>
            <div class="preview-item">
              <text-input size="md" placeholder="Error state" error errorMessage="This field is required"></text-input>
              <span>Error</span>
            </div>
          </div>
        `;
      case "search-input":
        return html`
          <div class="preview-sizes" style="gap: 2rem;">
            <div class="preview-item">
              <search-input size="sm" placeholder="Search..."></search-input>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <search-input size="md" placeholder="Search components..."></search-input>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <search-input size="lg" placeholder="Search..."></search-input>
              <span>Large</span>
            </div>
            <div class="preview-item">
              <search-input size="md" value="Searching" loading></search-input>
              <span>Loading</span>
            </div>
          </div>
        `;
      case "textarea-input":
        return html`
          <div class="preview-sizes" style="gap: 2rem;">
            <div class="preview-item">
              <textarea-input size="sm" placeholder="Small textarea" rows="2" style="width: 150px;"></textarea-input>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <textarea-input size="md" placeholder="Medium textarea" label="Description" rows="3" style="width: 200px;"></textarea-input>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <textarea-input size="lg" placeholder="Large textarea" rows="4" style="width: 250px;"></textarea-input>
              <span>Large</span>
            </div>
          </div>
        `;
      case "select-input":
        return html`
          <div class="preview-sizes" style="gap: 2rem;">
            <div class="preview-item">
              <select-input 
                size="sm" 
                placeholder="Select..." 
                .options=${[{ value: "1", label: "Option 1" }, { value: "2", label: "Option 2" }]}
                style="width: 120px;"
              ></select-input>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <select-input 
                size="md" 
                placeholder="Choose option..." 
                label="Category"
                .options=${[{ value: "a", label: "Alpha" }, { value: "b", label: "Beta" }, { value: "c", label: "Gamma" }]}
                style="width: 160px;"
              ></select-input>
              <span>Medium + Label</span>
            </div>
            <div class="preview-item">
              <select-input 
                size="lg" 
                placeholder="Select..." 
                .options=${[{ value: "x", label: "Extra Large" }, { value: "y", label: "Option Y" }]}
                style="width: 180px;"
              ></select-input>
              <span>Large</span>
            </div>
          </div>
        `;
      case "checkbox-input":
        return html`
          <div class="preview-sizes" style="gap: 2rem;">
            <div class="preview-item">
              <checkbox-input size="sm"></checkbox-input>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <checkbox-input size="md" label="Accept terms"></checkbox-input>
              <span>Medium + Label</span>
            </div>
            <div class="preview-item">
              <checkbox-input size="lg" checked></checkbox-input>
              <span>Large Checked</span>
            </div>
            <div class="preview-item">
              <checkbox-input size="md" disabled label="Disabled"></checkbox-input>
              <span>Disabled</span>
            </div>
          </div>
        `;
      case "toggle-input":
        return html`
          <div class="preview-sizes" style="gap: 2rem;">
            <div class="preview-item">
              <toggle-input size="sm"></toggle-input>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <toggle-input size="md" label="Enable feature"></toggle-input>
              <span>Medium + Label</span>
            </div>
            <div class="preview-item">
              <toggle-input size="lg" checked></toggle-input>
              <span>Large On</span>
            </div>
            <div class="preview-item">
              <toggle-input size="md" disabled label="Disabled"></toggle-input>
              <span>Disabled</span>
            </div>
          </div>
        `;
      // Sound types - event-based triggers
      case "ui-click":
      case "success-chime":
      case "error-tone":
      case "notification-ding":
      case "whoosh":
      case "confetti":
      case "error-file":
      case "firework":
      case "magic":
      case "success-file":
      case "warning":
        return html`
          <div style="display: flex; flex-direction: column; align-items: center; gap: 1rem;">
            ${this.renderSoundButton(tag as SoundType)}
            <span style="color: #6b7280; font-size: 0.875rem;">Click to play sound</span>
          </div>
        `;
      default:
        return html``;
    }
  }

  private renderCodeExample(component: ComponentInfo) {
    const tag = component.tag;
    
    // Build additional attributes based on component type
    let additionalAttrs = "";
    switch (tag) {
      case "wave-bar":
        additionalAttrs = ' bars="7"';
        break;
      case "loading-skeleton":
        additionalAttrs = ' variant="card"';
        break;
      case "primary-button":
      case "secondary-button":
      case "ghost-button":
      case "danger-button":
        return html`
          <div class="code-block">
            <span class="code-tag">&lt;${tag}</span>
            <span class="code-attr"> size</span>=<span class="code-value">"md"</span><span class="code-tag">&gt;</span>Click me<span class="code-tag">&lt;/${tag}&gt;</span>
          </div>
        `;
      case "icon-button":
        return html`
          <div class="code-block">
            <span class="code-tag">&lt;${tag}</span>
            <span class="code-attr"> size</span>=<span class="code-value">"md"</span>
            <span class="code-attr"> icon</span>=<span class="code-value">"+"</span>
            <span class="code-attr"> variant</span>=<span class="code-value">"primary"</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
          </div>
        `;
      case "button-group":
        return html`
          <div class="code-block">
            <span class="code-tag">&lt;${tag}</span>
            <span class="code-attr"> size</span>=<span class="code-value">"md"</span>
            <span class="code-attr"> value</span>=<span class="code-value">"option1"</span>
            <span class="code-attr"> variant</span>=<span class="code-value">"primary"</span>
            <span class="code-attr"> .options</span>=<span class="code-value">\${options}</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
          </div>
        `;
      case "text-input":
        return html`
          <div class="code-block">
            <span class="code-tag">&lt;${tag}</span>
            <span class="code-attr"> size</span>=<span class="code-value">"md"</span>
            <span class="code-attr"> label</span>=<span class="code-value">"Username"</span>
            <span class="code-attr"> placeholder</span>=<span class="code-value">"Enter username..."</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
          </div>
        `;
      case "search-input":
        return html`
          <div class="code-block">
            <span class="code-tag">&lt;${tag}</span>
            <span class="code-attr"> size</span>=<span class="code-value">"md"</span>
            <span class="code-attr"> placeholder</span>=<span class="code-value">"Search..."</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
          </div>
        `;
      case "textarea-input":
        return html`
          <div class="code-block">
            <span class="code-tag">&lt;${tag}</span>
            <span class="code-attr"> size</span>=<span class="code-value">"md"</span>
            <span class="code-attr"> label</span>=<span class="code-value">"Description"</span>
            <span class="code-attr"> rows</span>=<span class="code-value">"4"</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
          </div>
        `;
      case "select-input":
        return html`
          <div class="code-block">
            <span class="code-tag">&lt;${tag}</span>
            <span class="code-attr"> size</span>=<span class="code-value">"md"</span>
            <span class="code-attr"> label</span>=<span class="code-value">"Category"</span>
            <span class="code-attr"> .options</span>=<span class="code-value">\${options}</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
          </div>
        `;
      case "checkbox-input":
        return html`
          <div class="code-block">
            <span class="code-tag">&lt;${tag}</span>
            <span class="code-attr"> size</span>=<span class="code-value">"md"</span>
            <span class="code-attr"> label</span>=<span class="code-value">"Accept terms"</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
          </div>
        `;
      case "toggle-input":
        return html`
          <div class="code-block">
            <span class="code-tag">&lt;${tag}</span>
            <span class="code-attr"> size</span>=<span class="code-value">"md"</span>
            <span class="code-attr"> label</span>=<span class="code-value">"Enable feature"</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
          </div>
        `;
      case "fullpage-confetti":
        return html`
          <div class="code-block" style="display: flex; flex-direction: column; gap: 0.75rem;">
            <div><span style="color: #6b7280;">// HTML</span></div>
            <div>
              <span class="code-tag">&lt;${tag}</span>
              <span class="code-attr"> id</span>=<span class="code-value">"confetti"</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
            </div>
            <div style="margin-top: 0.5rem;"><span style="color: #6b7280;">// Trigger methods:</span></div>
            <div><span class="code-attr">document</span>.<span class="code-tag">getElementById</span>(<span class="code-value">'confetti'</span>).<span class="code-tag">trigger</span>()</div>
            <div><span class="code-attr">document</span>.<span class="code-tag">dispatchEvent</span>(<span class="code-value">new CustomEvent('trigger-confetti')</span>)</div>
            <div><span class="code-attr">window</span>.<span class="code-tag">triggerConfetti</span>()</div>
          </div>
        `;
      case "fullpage-fireworks":
        return html`
          <div class="code-block" style="display: flex; flex-direction: column; gap: 0.75rem;">
            <div><span style="color: #6b7280;">// HTML</span></div>
            <div>
              <span class="code-tag">&lt;${tag}</span>
              <span class="code-attr"> id</span>=<span class="code-value">"fireworks"</span>
              <span class="code-attr"> rockets</span>=<span class="code-value">"5"</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
            </div>
            <div style="margin-top: 0.5rem;"><span style="color: #6b7280;">// Trigger methods:</span></div>
            <div><span class="code-attr">document</span>.<span class="code-tag">getElementById</span>(<span class="code-value">'fireworks'</span>).<span class="code-tag">trigger</span>()</div>
            <div><span class="code-attr">document</span>.<span class="code-tag">dispatchEvent</span>(<span class="code-value">new CustomEvent('trigger-fireworks')</span>)</div>
            <div><span class="code-attr">window</span>.<span class="code-tag">triggerFireworks</span>()</div>
          </div>
        `;
      case "fullpage-aurora":
        return html`
          <div class="code-block" style="display: flex; flex-direction: column; gap: 0.75rem;">
            <div><span style="color: #6b7280;">// HTML</span></div>
            <div>
              <span class="code-tag">&lt;${tag}</span>
              <span class="code-attr"> id</span>=<span class="code-value">"aurora"</span>
              <span class="code-attr"> duration</span>=<span class="code-value">"4000"</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
            </div>
            <div style="margin-top: 0.5rem;"><span style="color: #6b7280;">// Trigger methods:</span></div>
            <div><span class="code-attr">document</span>.<span class="code-tag">getElementById</span>(<span class="code-value">'aurora'</span>).<span class="code-tag">trigger</span>()</div>
            <div><span class="code-attr">document</span>.<span class="code-tag">dispatchEvent</span>(<span class="code-value">new CustomEvent('trigger-aurora')</span>)</div>
            <div><span class="code-attr">window</span>.<span class="code-tag">triggerAurora</span>()</div>
          </div>
        `;
      case "fullpage-starfield":
        return html`
          <div class="code-block" style="display: flex; flex-direction: column; gap: 0.75rem;">
            <div><span style="color: #6b7280;">// HTML</span></div>
            <div>
              <span class="code-tag">&lt;${tag}</span>
              <span class="code-attr"> id</span>=<span class="code-value">"starfield"</span>
              <span class="code-attr"> duration</span>=<span class="code-value">"3000"</span><span class="code-tag">&gt;&lt;/${tag}&gt;</span>
            </div>
            <div style="margin-top: 0.5rem;"><span style="color: #6b7280;">// Trigger methods:</span></div>
            <div><span class="code-attr">document</span>.<span class="code-tag">getElementById</span>(<span class="code-value">'starfield'</span>).<span class="code-tag">trigger</span>()</div>
            <div><span class="code-attr">document</span>.<span class="code-tag">dispatchEvent</span>(<span class="code-value">new CustomEvent('trigger-starfield')</span>)</div>
            <div><span class="code-attr">window</span>.<span class="code-tag">triggerStarfield</span>()</div>
          </div>
        `;
      // Sound types - event-based system
      case "ui-click":
      case "success-chime":
      case "error-tone":
      case "notification-ding":
      case "whoosh":
      case "confetti":
      case "error-file":
      case "firework":
      case "magic":
      case "success-file":
      case "warning":
        return html`
          <div class="code-block" style="display: flex; flex-direction: column; gap: 0.75rem;">
            <div><span style="color: #6b7280;">// Import the sound utilities</span></div>
            <div><span class="code-attr">import</span> { <span class="code-tag">triggerSound</span>, <span class="code-tag">SoundController</span>, <span class="code-tag">dispatchSoundEvent</span> } <span class="code-attr">from</span> <span class="code-value">"./sounds"</span>;</div>
            <div style="margin-top: 0.5rem;"><span style="color: #6b7280;">// Option 1: Direct function call</span></div>
            <div><span class="code-tag">triggerSound</span>(<span class="code-value">"${tag}"</span>, <span class="code-value">0.5</span>);</div>
            <div style="margin-top: 0.5rem;"><span style="color: #6b7280;">// Option 2: Via SoundController (in LitElement)</span></div>
            <div><span class="code-attr">private</span> <span class="code-tag">soundController</span> = <span class="code-attr">new</span> <span class="code-tag">SoundController</span>(<span class="code-attr">this</span>);</div>
            <div><span class="code-attr">this</span>.<span class="code-tag">soundController</span>.<span class="code-tag">play</span>(<span class="code-value">"${tag}"</span>);</div>
            <div style="margin-top: 0.5rem;"><span style="color: #6b7280;">// Option 3: Dispatch event (bubbles up to listener)</span></div>
            <div><span class="code-tag">dispatchSoundEvent</span>(<span class="code-attr">this</span>, <span class="code-value">"${tag}"</span>, <span class="code-value">0.5</span>);</div>
          </div>
        `;
    }
    
    return html`
      <div class="code-block">
        <span class="code-tag">&lt;${tag}</span>
        <span class="code-attr"> size</span>=<span class="code-value">"md"</span>${additionalAttrs}<span class="code-tag">&gt;&lt;/${tag}&gt;</span>
      </div>
    `;
  }

  private renderMain() {
    const components = this.getComponentsForTab();
    const component = components.find((c) => c.tag === this.selectedComponent);

    if (!component) {
      return html`
        <div class="main">
          <div class="empty">
            <div class="empty-icon">📦</div>
            <p>Select a component from the sidebar</p>
          </div>
        </div>
      `;
    }

    return html`
      <div class="main">
        <div class="main-header">
          <h2>${component.name}</h2>
          <p>${component.description}</p>
        </div>

        <div class="preview">
          ${this.renderComponentPreview(component.tag)}
        </div>

        <div class="props">
          <h3>Properties</h3>
          <table class="props-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Type</th>
                <th>Default</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              ${component.props.map(
                (prop) => html`
                  <tr>
                    <td class="prop-name">${prop.name}</td>
                    <td class="prop-type">${prop.type}</td>
                    <td class="prop-default">${prop.default || "-"}</td>
                    <td>${prop.description}</td>
                  </tr>
                `
              )}
            </tbody>
          </table>
        </div>

        <div class="code">
          <h3>Usage</h3>
          ${this.renderCodeExample(component)}
        </div>
      </div>
    `;
  }

  connectedCallback() {
    super.connectedCallback();
    if (this.loadingComponents.length > 0) {
      this.selectedComponent = this.loadingComponents[0].tag;
    }
  }

  render() {
    return html`
      <div class="page">
        <div class="header">
          <h1>Components</h1>
          <p>Reusable UI components for the web app</p>
        </div>

        <div class="tabs">
          <button
            class="tab ${this.activeTab === "loading" ? "active" : ""}"
            @click=${() => {
              this.activeTab = "loading";
              this.selectedComponent = this.loadingComponents[0]?.tag || null;
            }}
          >
            Loading Indicators
          </button>
          <button
            class="tab ${this.activeTab === "speceffects" ? "active" : ""}"
            @click=${() => {
              this.activeTab = "speceffects";
              this.selectedComponent = this.specEffectsComponents[0]?.tag || null;
            }}
          >
            Spec Effects
          </button>
          <button
            class="tab ${this.activeTab === "fullpageeffects" ? "active" : ""}"
            @click=${() => {
              this.activeTab = "fullpageeffects";
              this.selectedComponent = this.fullPageEffectsComponents[0]?.tag || null;
            }}
          >
            Full Page Spec Effects
          </button>
          <button
            class="tab ${this.activeTab === "buttons" ? "active" : ""}"
            @click=${() => {
              this.activeTab = "buttons";
              this.selectedComponent = this.buttonComponents[0]?.tag || null;
            }}
          >
            Buttons
          </button>
          <button
            class="tab ${this.activeTab === "inputs" ? "active" : ""}"
            @click=${() => {
              this.activeTab = "inputs";
              this.selectedComponent = this.inputComponents[0]?.tag || null;
            }}
          >
            Inputs
          </button>
          <button
            class="tab ${this.activeTab === "sounds" ? "active" : ""}"
            @click=${() => {
              this.activeTab = "sounds";
              this.selectedComponent = this.soundComponents[0]?.tag || null;
            }}
          >
            Sounds
          </button>
          <button class="tab disabled" disabled>Feedback</button>
        </div>

        <div class="content">
          ${this.renderSidebar()}
          ${this.renderMain()}
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "components-page": ComponentsPage;
  }
}
