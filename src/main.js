import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles/main.css";
import { onAiPromptDebug } from "./utils/tauri";

const app = createApp(App);
app.use(createPinia());
app.mount("#app");

// 监听后端发送的 AI 提示词，打印到前端控制台
onAiPromptDebug((data) => {
  console.log("\n========== AI Prompt ==========");
  console.log("Provider:", data.provider);
  console.log("Model:", data.model);
  console.log("Prompt length:", data.prompt_length, "chars");
  console.log("-------------------------------");
  console.log(data.prompt);
  console.log("===============================\n");
});
