/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        strapi: {
          primary: '#4945ff',
          'primary-hover': '#2f27fd',
          'primary-light': '#f0f0ff',
          bg: '#f6f6f9',
          card: '#ffffff',
          sidebar: '#212134',
          'sidebar-active': '#181826',
          border: '#eaeaef',
          text: '#32324d',
          subtext: '#666687',
          neutral100: '#f6f6f9',
          neutral150: '#eaeaef',
          neutral200: '#dcdce4',
          neutral600: '#666687',
          neutral800: '#32324d',
          neutral900: '#212134',
        }
      }
    },
  },
  plugins: [],
}
