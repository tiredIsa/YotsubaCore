<script setup lang="ts">
import ProfilesList from "../components/layout/profiles/ProfilesList.vue";
import ProfilesImportShare from "../components/layout/profiles/ProfilesImportShare.vue";
import ProfilesImportJson from "../components/layout/profiles/ProfilesImportJson.vue";
import ProfilesAlerts from "../components/layout/profiles/ProfilesAlerts.vue";
</script>

<template>
  <section class="flex flex-col gap-6">
    <ProfilesList />
    <div class="grid gap-6 lg:grid-cols-2">
      <ProfilesImportShare />
      <ProfilesImportJson />
    </div>
    <ProfilesAlerts />
  </section>
</template>
