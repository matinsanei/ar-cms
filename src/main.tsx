// Polyfill process in module scope before anything else runs
if (typeof window !== 'undefined') {
  (window as any).process = (window as any).process || {};
  (window as any).process.env = {
    NODE_ENV: 'development',
    STRAPI_ADMIN_BACKEND_URL: '',
    STRAPI_ADMIN_AUTH_COOKIE_NAME: 'jwtToken',
    STRAPI_ADMIN_AUTH_COOKIE_PATH: '/',
    STRAPI_ADMIN_AUTH_COOKIE_DOMAIN: '',
    STRAPI_TELEMETRY_DISABLED: 'true',
    ...((window as any).process.env || {}),
  };
  (window as any).process.platform = 'browser';
  (window as any).process.version = 'v20.0.0';
  (window as any).global = window;

  // Silence known third-party a11y & dev noise from Radix UI / Strapi
  const _origError = console.error;
  console.error = function (...args: any[]) {
    if (
      typeof args[0] === 'string' &&
      (args[0].includes('requires a `DialogTitle`') ||
        (args[0].includes('DialogTitle') && args[0].includes('radix-ui')))
    ) {
      return;
    }
    _origError.apply(console, args);
  };

  const _origWarn = console.warn;
  console.warn = function (...args: any[]) {
    if (
      typeof args[0] === 'string' &&
      (args[0].includes('Missing `Description`') ||
        args[0].includes('aria-describedby') ||
        args[0].includes('useRBAC: The first argument should be an array of permissions') ||
        args[0].includes('React Router Future Flag Warning'))
    ) {
      return;
    }
    _origWarn.apply(console, args);
  };
}

import { renderAdmin } from '@strapi/admin/strapi-admin';
import contentManager from '@strapi/content-manager/strapi-admin';
import contentTypeBuilder from '@strapi/content-type-builder/strapi-admin';

// Force dark mode as the default experience if not previously chosen
if (typeof window !== 'undefined' && !localStorage.getItem('STRAPI_THEME')) {
  localStorage.setItem('STRAPI_THEME', 'dark');
}

// Sleek Modern Light Theme with Royal Purple / Violet Accents
const customLightTheme = {
  colors: {
    // Surface backgrounds - Clean White & GitHub Light Grays
    neutral0: '#ffffff',      // Primary card, panel, modal background
    neutral100: '#f6f8fa',    // Global app page background (GitHub light canvas)
    neutral150: '#eaeef2',    // Elevated surfaces, input hover states
    neutral200: '#d0d7de',    // Subtle borders & dividers
    neutral300: '#afb8c1',    // Secondary borders
    neutral400: '#8c959f',    // Placeholder text, icons
    neutral500: '#6e7781',    // Muted labels, secondary descriptions
    neutral600: '#424a53',    // Body text
    neutral700: '#24292f',    // Emphasized body text
    neutral800: '#1f2328',    // Headings
    neutral900: '#1f2328',    // Main headings
    neutral1000: '#000000',   // Maximum contrast text

    // Primary - Royal Violet / Purple
    primary100: '#f5f3ff',    // Very soft purple badge background
    primary200: '#ede9fe',    // Soft purple outline
    primary500: '#8b5cf6',    // Purple interactive hover
    primary600: '#7c3aed',    // Royal Violet primary action buttons
    primary700: '#6d28d9',    // Active / pressed primary

    // Button colors
    buttonPrimary500: '#8b5cf6',
    buttonPrimary600: '#7c3aed',
    buttonNeutral0: '#ffffff',

    // Success - Emerald
    success100: '#dafbe1',
    success200: '#aceebb',
    success500: '#2ea043',
    success600: '#238636',
    success700: '#1a7f37',

    // Danger - Vibrant Red
    danger100: '#ffebe9',
    danger200: '#ffcecb',
    danger500: '#cf222e',
    danger600: '#a40e26',
    danger700: '#67060c',

    // Warning - Warm Gold
    warning100: '#fff8c5',
    warning200: '#fae17d',
    warning500: '#bf8700',
    warning600: '#9a6700',
    warning700: '#7d4e00',

    // Alternative - Soft Purple
    alternative100: '#f5f3ff',
    alternative200: '#ede9fe',
    alternative500: '#8b5cf6',
    alternative600: '#7c3aed',
    alternative700: '#6d28d9',

    // Secondary - Slate Gray
    secondary100: '#f6f8fa',
    secondary200: '#eaeef2',
    secondary500: '#6e7781',
    secondary600: '#424a53',
    secondary700: '#24292f',
  },
  shadows: {
    tableShadow: '0 1px 3px 0 rgba(31, 35, 40, 0.12), 0 1px 2px -1px rgba(31, 35, 40, 0.12)',
    popupShadow: '0 8px 24px 0 rgba(31, 35, 40, 0.16), 0 2px 6px 0 rgba(31, 35, 40, 0.08)',
    filterShadow: '0 1px 3px rgba(31, 35, 40, 0.08)',
  },
};

// GitHub & Discord Sleek Charcoal / Matte Gray Dark Theme with Royal Purple Accents
const customDarkTheme = {
  colors: {
    // Surface backgrounds - GitHub Dark Canvas & Discord Matte Charcoal
    neutral0: '#161b22',      // Primary card, panel, modal background (GitHub Dark surface / Discord #2b2d31)
    neutral100: '#0d1117',    // Global app page background (GitHub Dark canvas / Discord #1e1f22)
    neutral150: '#21262d',    // Elevated surfaces, input background, hover states (GitHub Dark elevated / Discord #313338)
    neutral200: '#30363d',    // Sharp subtle borders & dividers (GitHub Dark border / Discord #383a40)
    neutral300: '#3c444d',    // Secondary borders / inactive tabs (Discord #4e5058)
    neutral400: '#6e7681',    // Placeholder text, muted icons (Discord #80848e)
    neutral500: '#8b949e',    // Muted labels, secondary descriptions (Discord #949ba4)
    neutral600: '#c9d1d9',    // Regular body text (Discord #dbdee1)
    neutral700: '#e6edf3',    // High contrast body text (GitHub text-primary)
    neutral800: '#f0f6fc',    // Subheadings and emphasized text
    neutral900: '#ffffff',    // Main headings (pure white)
    neutral1000: '#ffffff',   // Maximum contrast text

    // Primary - Vibrant Discord / Royal Violet Purple
    primary100: '#241e3d',    // Subtle purple/charcoal tint for active badges
    primary200: '#3c2e64',    // Outline / selection border
    primary500: '#8b5cf6',    // Purple hover state
    primary600: '#7c3aed',    // Royal Violet primary action button
    primary700: '#6d28d9',    // Active / pressed primary action

    // Button colors
    buttonPrimary500: '#8b5cf6',
    buttonPrimary600: '#7c3aed',
    buttonNeutral0: '#ffffff',

    // Success - GitHub Green
    success100: '#13231b',
    success200: '#1c3d2a',
    success500: '#2ea043',
    success600: '#238636',
    success700: '#7ee787',

    // Danger - GitHub Dark Crimson / Coral
    danger100: '#2c1517',
    danger200: '#491b1d',
    danger500: '#f85149',
    danger600: '#da3633',
    danger700: '#ffa198',

    // Warning - GitHub Dark Amber / Gold
    warning100: '#2e1f0c',
    warning200: '#4d3211',
    warning500: '#d29922',
    warning600: '#bb8009',
    warning700: '#f8e3a1',

    // Alternative - Discord Soft Purple
    alternative100: '#26203a',
    alternative200: '#382e59',
    alternative500: '#a78bfa',
    alternative600: '#8b5cf6',
    alternative700: '#ddd6fe',

    // Secondary - Discord / GitHub Neutral Gray
    secondary100: '#161b22',
    secondary200: '#21262d',
    secondary500: '#6e7681',
    secondary600: '#484f58',
    secondary700: '#8b949e',
  },
  shadows: {
    tableShadow: '0 4px 16px 0 rgba(1, 4, 9, 0.6), 0 1px 3px 0 rgba(1, 4, 9, 0.4)',
    popupShadow: '0 12px 28px 0 rgba(1, 4, 9, 0.8), 0 2px 4px 0 rgba(1, 4, 9, 0.5)',
    filterShadow: '0 1px 3px rgba(1, 4, 9, 0.4)',
  },
};

const container = document.getElementById('strapi');

if (container) {
  renderAdmin(container, {
    plugins: {
      'content-manager': contentManager,
      'content-type-builder': contentTypeBuilder,
    },
    customisations: {
      auth: {
        logo: '/logo.png',
      },
      menu: {
        logo: '/logo-menu.png',
      },
      config: {
        theme: {
          light: customLightTheme,
          dark: customDarkTheme,
        },
        auth: {
          logo: '/logo.png',
        },
        menu: {
          logo: '/logo-menu.png',
        },
        locales: ['en'],
        notifications: {
          releases: false,
        },
        translations: {
          en: {
            'Settings.application.strapi-version': 'AR-CMS Engine Version',
            'Settings.application.node-version': 'Runtime Architecture',
            'Settings.application.header': 'AR-CMS Overview',
            'Settings.application.title': 'AR-CMS System Overview',
            'Settings.application.description': 'High-performance modular document headless CMS engine',
            'Settings.application.customization': 'Branding & Customization',
            'Settings.application.plan.title': 'AR-CMS Architecture',
            'Settings.application.plan.current': 'AR-CMS Unified Engine',
            'Settings.application.plan.see-all-plans': 'Documentation',
            'Settings.application.support.title': 'System Status',
            'Settings.application.support.subtitle.community': 'AR-CMS zero-restart hot-reload runtime engine is active and healthy.',
            'global.strapi': 'AR-CMS',
            'global.marketplace': '',
            'Settings.permissions': 'Administration Panel',
            'Settings.permissions.users.strapi-super-admin': 'Super Admin',
            'Settings.permissions.users.strapi-editor': 'Editor',
            'Settings.permissions.users.strapi-author': 'Author',
            'Auth.form.welcome.title': 'Welcome to AR-CMS!',
            'Auth.form.welcome.subtitle': 'Log in to your AR-CMS account',
          },
        },
      },
    },
  });
}
