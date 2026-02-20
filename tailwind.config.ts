import type { Config } from 'tailwindcss';

export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        'prayer-fajr': '#ff9800',
        'prayer-sunrise': '#ffeb3b',
        'prayer-dhuhr': '#ff5722',
        'prayer-asr': '#9c27b0',
        'prayer-maghrib': '#f44336',
        'prayer-isha': '#3f51b5',
      },
      fontFamily: {
        arabic: ['"Segoe UI"', 'Tahoma', 'Geneva', 'Verdana', '"Amiri"', '"Traditional Arabic"', 'serif'],
      }
    }
  },
  plugins: []
} satisfies Config;
