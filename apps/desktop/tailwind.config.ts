import type { Config } from "tailwindcss";
import forms from "@tailwindcss/forms";

const config: Config = {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        brand: {
          500: "#8b5cf6",
          600: "#7c3aed",
        },
      },
    },
  },
  plugins: [forms],
};

export default config;
