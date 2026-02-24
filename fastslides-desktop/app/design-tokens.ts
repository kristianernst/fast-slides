export const DESIGN_TOKENS = {
  colors: {
    primary: "#6681AE",
    primaryHover: "#748FB9",
    primaryActive: "#567094",
    primaryScale: {
      50: "#EDF2F8",
      100: "#D8E2F0",
      200: "#BDCDE2",
      300: "#9FB6D4",
      400: "#809DC4",
      500: "#6681AE",
      600: "#587198",
      700: "#4A5F81",
      800: "#3C4D69",
      900: "#2B374D",
    },
    text: {
      primary: "#EFEFEB",
      secondary: "#C4C3BF",
      tertiary: "#9E9D98",
      disabled: "rgba(239, 239, 235, 0.45)",
      inverse: "#1F1E1B",
    },
    bg: {
      base: "#272623",
      elevated: "#302F2C",
      hover: "#43423E",
      active: "#4F4E49",
      canvas: "#1F1E1C",
      overlay: "rgba(9, 9, 8, 0.68)",
    },
    surface: {
      default: "#302F2C",
      hover: "#43423E",
      raised: "#393834",
    },
    border: "#4F4E49",
    borderSubtle: "#43423E",
    borderFocus: "#6681AE",
    status: {
      success: "#63B18A",
      warning: "#E1B86F",
      error: "#D68080",
      info: "#6681AE",
    },
  },

  typography: {
    fontFamily: {
      sans: '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
      mono: '"Fira Code", "Courier New", monospace',
    },

    fontSize: {
      xs: "0.75rem",
      sm: "0.875rem",
      base: "1rem",
      lg: "1.125rem",
      xl: "1.25rem",
      "2xl": "1.5rem",
      "3xl": "1.875rem",
      "4xl": "2.25rem",
      "5xl": "3rem",
    },

    fontWeight: {
      light: 300,
      normal: 400,
      medium: 500,
      semibold: 600,
      bold: 700,
    },

    lineHeight: {
      tight: 1.25,
      normal: 1.5,
      relaxed: 1.75,
      loose: 2,
    },

    letterSpacing: {
      tighter: "-0.05em",
      tight: "-0.025em",
      normal: "0em",
      wide: "0.025em",
      wider: "0.05em",
      widest: "0.1em",
    },
  },

  spacing: {
    px: "1px",
    0.5: "0.125rem",
    1: "0.25rem",
    1.5: "0.375rem",
    2: "0.5rem",
    2.5: "0.625rem",
    3: "0.75rem",
    3.5: "0.875rem",
    4: "1rem",
    5: "1.25rem",
    6: "1.5rem",
    7: "1.75rem",
    8: "2rem",
    9: "2.25rem",
    10: "2.5rem",
    12: "3rem",
    14: "3.5rem",
    16: "4rem",
    20: "5rem",
    24: "6rem",
  },

  borderRadius: {
    none: "0",
    sm: "0.375rem",
    md: "0.5rem",
    lg: "0.625rem",
    xl: "0.75rem",
    "2xl": "1rem",
    "3xl": "1.5rem",
    full: "9999px",
  },

  shadow: {
    none: "none",
    sm: "0 1px 2px 0 rgba(0, 0, 0, 0.05)",
    base: "0 1px 3px 0 rgba(0, 0, 0, 0.1)",
    md: "0 4px 6px -1px rgba(0, 0, 0, 0.1)",
    lg: "0 10px 15px -3px rgba(0, 0, 0, 0.1)",
    xl: "0 20px 25px -5px rgba(0, 0, 0, 0.1)",
    "2xl": "0 25px 50px -12px rgba(0, 0, 0, 0.25)",
    glow: "0 0 20px rgba(102, 129, 174, 0.28)",
  },

  transition: {
    fast: "120ms ease-out",
    base: "160ms ease-out",
    slow: "220ms ease-out",

    easing: {
      linear: "linear",
      in: "cubic-bezier(0.4, 0, 1, 1)",
      out: "cubic-bezier(0, 0, 0.2, 1)",
      inOut: "cubic-bezier(0.4, 0, 0.2, 1)",
      smooth: "cubic-bezier(0.23, 1, 0.32, 1)",
    },
  },

  sizing: {
    sidebarWidth: "280px",
    sidebarCollapsedWidth: "0px",
    headerHeight: "56px",
    footerHeight: "32px",
    contentMaxWidth: "1400px",
  },

  zIndex: {
    base: 0,
    dropdown: 10,
    sticky: 20,
    fixed: 30,
    modal: 40,
    popover: 50,
    tooltip: 60,
  },

  opacity: {
    0: "0",
    5: "0.05",
    10: "0.1",
    20: "0.2",
    25: "0.25",
    30: "0.3",
    40: "0.4",
    50: "0.5",
    60: "0.6",
    70: "0.7",
    75: "0.75",
    80: "0.8",
    90: "0.9",
    95: "0.95",
    100: "1",
  },
} as const;

export const COMPONENT_TOKENS = {
  button: {
    primary: {
      bg: DESIGN_TOKENS.colors.primary,
      bgHover: DESIGN_TOKENS.colors.primaryHover,
      text: "#FFFFFF",
      border: "transparent",
    },
    secondary: {
      bg: DESIGN_TOKENS.colors.bg.elevated,
      bgHover: DESIGN_TOKENS.colors.bg.hover,
      text: DESIGN_TOKENS.colors.text.primary,
      border: DESIGN_TOKENS.colors.border,
    },
  },

  sidebar: {
    bg: DESIGN_TOKENS.colors.bg.elevated,
    border: DESIGN_TOKENS.colors.border,
    itemHover: DESIGN_TOKENS.colors.bg.hover,
    itemActive: DESIGN_TOKENS.colors.bg.active,
  },

  input: {
    bg: DESIGN_TOKENS.colors.bg.base,
    border: DESIGN_TOKENS.colors.border,
    borderFocus: DESIGN_TOKENS.colors.borderFocus,
    text: DESIGN_TOKENS.colors.text.primary,
    placeholder: DESIGN_TOKENS.colors.text.secondary,
  },
} as const;
