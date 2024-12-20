// AlertBanner Component
class AlertBanner extends HTMLElement {
  constructor() {
    super();
    this.type = this.getAttribute('type') || '';
  }

  connectedCallback() {
    this.classList.add('alert-banner');
    if (this.type) {
      this.classList.add(this.type.toLowerCase());
    }
  }
}

// Button Component
class Button extends HTMLElement {
  constructor() {
    super();
    this.variant = this.getAttribute('variant') || '';
  }

  connectedCallback() {
    this.classList.add('button');
    if (this.variant) {
      this.classList.add(this.variant.toLowerCase());
    }
  }
}

// Card Component
class Card extends HTMLElement {
  constructor() {
    super();
  }

  connectedCallback() {
    this.classList.add('card');
    
    // Create sections if they don't exist
    if (!this.querySelector('.card-header')) {
      const header = document.createElement('div');
      header.classList.add('card-header');
      this.prepend(header);
    }
    
    if (!this.querySelector('.card-content')) {
      const content = document.createElement('div');
      content.classList.add('card-content');
      this.insertBefore(content, this.querySelector('.card-footer'));
    }
  }
}

// Text Component
class Text extends HTMLElement {
  constructor() {
    super();
    this.variant = this.getAttribute('variant') || 'body';
  }

  connectedCallback() {
    this.classList.add('text');
    this.classList.add(this.variant);
  }
}

// TextArea Component
class TextArea extends HTMLTextAreaElement {
  constructor() {
    super();
  }

  connectedCallback() {
    this.classList.add('textarea');
  }
}

// DataTable Component
class DataTable extends HTMLTableElement {
  constructor() {
    super();
  }

  connectedCallback() {
    this.classList.add('data-table');
  }
}

// Register Custom Elements
customElements.define('alert-banner', AlertBanner);
customElements.define('custom-button', Button);
customElements.define('custom-card', Card);
customElements.define('custom-text', Text);
customElements.define('custom-textarea', TextArea, { extends: 'textarea' });
customElements.define('data-table', DataTable, { extends: 'table' });