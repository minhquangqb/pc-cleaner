import { createApp } from "vue";
import App from "./App.vue";
import { i18n, currentLocale } from "./i18n";
import { setAppLanguage } from "./api";
import "./style.css";

document.documentElement.lang = currentLocale();
// Keep the Rust side (tray menu, notifications) in the same language.
setAppLanguage(currentLocale()).catch(() => {});

createApp(App).use(i18n).mount("#app");
