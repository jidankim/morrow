# Morrow Design System

## 1. Atmosphere & Identity

Morrow is a quiet Mac utility for private scheduling review: compact, readable, and calm under failure. The signature is a restrained status rail: status, controls, and settings remain visible without turning the product into a dashboard.

## 2. Color

### Palette

| Role | Token | Light | Dark | Usage |
|------|-------|-------|------|-------|
| Surface/primary | --surface-primary | #F7F8F6 | #101412 | App background |
| Surface/secondary | --surface-secondary | #FFFFFF | #171C19 | Panels |
| Surface/elevated | --surface-elevated | #FFFFFF | #202620 | Popovers and controls |
| Text/primary | --text-primary | #17211B | #F4F7F1 | Headlines and body |
| Text/secondary | --text-secondary | #526158 | #AAB6AD | Supporting text |
| Text/tertiary | --text-tertiary | #7A877E | #717C73 | Disabled text |
| Border/default | --border-default | #DDE4DC | #2C342E | Panel outlines |
| Border/subtle | --border-subtle | #ECF0EA | #222923 | Dividers |
| Accent/primary | --accent-primary | #166B4F | #4CC38A | Primary commands |
| Accent/hover | --accent-hover | #10583F | #6DD3A0 | Hover state |
| Status/success | --status-success | #1F7A52 | #4CC38A | Ready or healthy |
| Status/warning | --status-warning | #A16207 | #F2B84B | Sync Now disabled |
| Status/error | --status-error | #B42318 | #FF7A6D | Error |
| Status/info | --status-info | #2D5B8A | #79B7F2 | Informational |

### Rules

- Accent is reserved for commands and focus.
- Status colors only communicate app state.
- New colors must be added here before use.

## 3. Typography

### Scale

| Level | Size | Weight | Line Height | Tracking | Usage |
|-------|------|--------|-------------|----------|-------|
| H1 | 28px | 650 | 1.2 | 0 | App title |
| H2 | 20px | 650 | 1.3 | 0 | Panel headings |
| H3 | 16px | 650 | 1.35 | 0 | Group headings |
| Body | 15px | 400 | 1.55 | 0 | Default text |
| Body/sm | 13px | 400 | 1.45 | 0 | Secondary info |
| Caption | 12px | 600 | 1.4 | 0 | Labels and metadata |

### Font Stack

- Primary: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
- Mono: "SFMono-Regular", Consolas, monospace

### Rules

- Body text never renders below 13px in this app shell.
- Use mono only for IDs, paths, and technical state labels.

## 4. Spacing & Layout

### Base Unit

All spacing derives from 4px.

| Token | Value | Usage |
|-------|-------|-------|
| --space-1 | 4px | Tight inline gaps |
| --space-2 | 8px | Compact control gaps |
| --space-3 | 12px | Field padding |
| --space-4 | 16px | Panel inner spacing |
| --space-5 | 20px | Cluster spacing |
| --space-6 | 24px | Page padding |
| --space-8 | 32px | Major groups |

### Grid

- Max content width: 1040px
- Column system: fixed status rail plus flexible content
- Breakpoints: compact below 720px, full shell above 960px

### Rules

- Status and action controls use stable dimensions to avoid layout shifts.
- App sections are unframed layouts or single-level panels, never nested cards.

## 5. Components

### Status Pill

- **Structure**: inline status dot plus label.
- **Variants**: ready, sync-now-disabled, error.
- **Spacing**: --space-2 gap, --space-3 horizontal padding.
- **States**: text changes by app mode.
- **Accessibility**: exposed as status text.
- **Motion**: color transitions over 150ms.

### Command Button

- **Structure**: button with optional icon and text.
- **Variants**: primary, secondary, danger.
- **Spacing**: --space-2 icon gap, --space-3 inline padding.
- **States**: hover, active, focus, disabled.
- **Accessibility**: native button, visible focus ring.
- **Motion**: transform and color transitions only.

### Automatic Sync Control

- **Structure**: compact status header, command toggle, whole-minute interval number field, and three metadata rows.
- **Variants**: off, enabled, running, cooling down, needs action.
- **Spacing**: --space-3 control gap, --space-4 metadata gap, --space-5 section padding.
- **States**: toggle and valid interval changes persist durable scheduler configuration; invalid interval drafts reset on blur.
- **Accessibility**: native button, native number input with minimum 1 minute, status exposed with role=status.
- **Motion**: button transform and color transitions only.

## 6. Motion & Interaction

### Timing

| Type | Duration | Easing | Usage |
|------|----------|--------|-------|
| Micro | 120ms | ease-out | Button press |
| Standard | 180ms | ease-in-out | Route and status transitions |

### Rules

- Only animate transform and opacity.
- Every interactive element has hover, active, focus, and disabled styling.
- Respect `prefers-reduced-motion`.

## 7. Depth & Surface

### Strategy

Mixed, with borders for structure and subtle tonal shifts for state.

| Type | Value | Usage |
|------|-------|-------|
| Default border | 1px solid var(--border-default) | Panels and controls |
| Subtle border | 1px solid var(--border-subtle) | Dividers |
| Panel radius | 8px | Panels |
| Control radius | 6px | Buttons and inputs |
