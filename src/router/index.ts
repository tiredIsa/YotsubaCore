import { createRouter, createWebHashHistory } from "vue-router";
import DashboardView from "../view/DashboardView.vue";
import AppsView from "../view/AppsView.vue";
import ProfilesView from "../view/ProfilesView.vue";
import LogsView from "../view/LogsView.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      redirect: "/dashboard",
    },
    {
      path: "/dashboard",
      name: "dashboard",
      component: DashboardView,
    },
    {
      path: "/apps",
      name: "apps",
      component: AppsView,
    },
    {
      path: "/profiles",
      name: "profiles",
      component: ProfilesView,
    },
    {
      path: "/logs",
      name: "logs",
      component: LogsView,
    },
  ],
  scrollBehavior() {
    return { top: 0 };
  },
});

export default router;
