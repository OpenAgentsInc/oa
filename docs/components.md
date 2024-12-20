# OpenAgents Web Components

A collection of vanilla web components that can be used in any HTML project without framework dependencies. These components are adapted from the www-sacred React components to provide a consistent design system.

## Installation

1. Include the required files in your HTML:

```html
<!-- Add to your <head> section -->
<link rel="stylesheet" href="/components/styles.css">
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap" rel="stylesheet">

<!-- Add before closing </body> tag -->
<script src="/components/components.js"></script>
```

## Available Components

### AlertBanner

A banner for displaying status messages or alerts.

```html
<alert-banner>Default message</alert-banner>
<alert-banner type="success">Success message</alert-banner>
<alert-banner type="error">Error message</alert-banner>
```

Attributes:
- `type`: Optional. Values: `"success"` | `"error"`

### Button

A customizable button component.

```html
<custom-button>Default Button</custom-button>
<custom-button variant="primary">Primary Button</custom-button>
```

Attributes:
- `variant`: Optional. Values: `"primary"`

### Card

A container component for grouping related content.

```html
<custom-card>
  <div class="card-header">Card Title</div>
  <div class="card-content">Main content goes here</div>
  <div class="card-footer">Footer content</div>
</custom-card>
```

Sections:
- `.card-header`: Optional. Card title or header content
- `.card-content`: Main content area
- `.card-footer`: Optional. Footer content or actions

### Text

A text component with predefined styles for different typography needs.

```html
<custom-text variant="h1">Heading 1</custom-text>
<custom-text variant="h2">Heading 2</custom-text>
<custom-text variant="h3">Heading 3</custom-text>
<custom-text>Regular body text</custom-text>
<custom-text variant="small">Small text</custom-text>
```

Attributes:
- `variant`: Optional. Values: `"h1"` | `"h2"` | `"h3"` | `"body"` | `"small"`

### TextArea

An enhanced textarea element with consistent styling.

```html
<textarea is="custom-textarea" placeholder="Enter text here..."></textarea>
```

Supports all standard textarea attributes.

### DataTable

A styled table component for displaying structured data.

```html
<table is="data-table">
  <thead>
    <tr>
      <th>Header 1</th>
      <th>Header 2</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Data 1</td>
      <td>Data 2</td>
    </tr>
  </tbody>
</table>
```

## CSS Variables

The components use CSS variables for consistent theming. You can override these variables to customize the appearance:

```css
:root {
  --theme-success: #2ecc71;
  --theme-success-subdued: #27ae60;
  --theme-error: #e74c3c;
  --theme-error-subdued: #c0392b;
  --theme-border: #e0e0e0;
  --theme-border-subdued: #bdc3c7;
  --theme-line-height-base: 1.5;
  --theme-font-family: 'JetBrains Mono', monospace;
  --theme-text-color: #2c3e50;
  --theme-background: #ffffff;
  --theme-primary: #3498db;
  --theme-primary-subdued: #2980b9;
}
```

## Example Page

An example page showcasing all components is available at `/components/index.html`. You can use this as a reference for component usage and styling.

## Browser Support

These components use the Web Components API and Custom Elements. They are supported in all modern browsers. For older browsers, you may need to include the Custom Elements polyfill.

## Development

The components are built using vanilla JavaScript and the Web Components API. Each component is defined as a custom element that extends the appropriate HTML element.

Files:
- `components.js`: Contains the JavaScript class definitions for all components
- `styles.css`: Contains all component styles
- `index.html`: Example usage and documentation

To add new components:

1. Add the component class to `components.js`
2. Register it using `customElements.define()`
3. Add corresponding styles to `styles.css`
4. Update this documentation with usage examples