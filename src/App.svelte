<script lang="ts">
  import { setContext, onMount, type ComponentType } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getVersion } from "@tauri-apps/api/app";
  import { _ } from "svelte-i18n";
  import Sidebar from "./components/Sidebar.svelte";
  import AboutDialog from "./components/AboutDialog.svelte";
  import Repositories from "./views/Repositories.svelte";
  import SourcesGenerator from "./views/SourcesGenerator.svelte";
  import GpgKeys from "./views/GpgKeys.svelte";
  import { ListTree, FileCog, KeyRound } from "lucide-svelte";

  // No separate welcome screen, unlike the 2.x/1.x Python version — the app
  // opens straight into the Repositories section.
  $: tabs = [
    {
      id: "repositories",
      icon: ListTree,
      label: $_("app.tabs.repositories.label"),
      title: $_("app.tabs.repositories.title"),
      description: $_("app.tabs.repositories.description"),
    },
    {
      id: "sourcesGenerator",
      icon: FileCog,
      label: $_("app.tabs.sourcesGenerator.label"),
      title: $_("app.tabs.sourcesGenerator.title"),
      description: $_("app.tabs.sourcesGenerator.description"),
    },
    {
      id: "gpgKeys",
      icon: KeyRound,
      label: $_("app.tabs.gpgKeys.label"),
      title: $_("app.tabs.gpgKeys.title"),
      description: $_("app.tabs.gpgKeys.description"),
    },
  ];

  const views: Record<string, ComponentType> = {
    repositories: Repositories,
    sourcesGenerator: SourcesGenerator,
    gpgKeys: GpgKeys,
  };

  let active = "repositories";
  let appVersion = "";

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch {
      appVersion = "";
    }
  });

  function select(id: string) {
    active = id;
    document.getElementById("main-scroll")?.scrollTo({ top: 0 });
  }

  setContext("navigate", select);
  setContext("openAbout", () => (aboutOpen = true));

  $: activeTab = tabs.find((t) => t.id === active)!;
  $: ActiveView = views[active];

  let aboutOpen = false;

  // F1 -> About, Ctrl+Q -> quit, Ctrl+Tab / Ctrl+Shift+Tab -> cycle sections
  // (ported from the 2.x shortcuts: F1, Ctrl+Shift+Tab).
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "F1") {
      e.preventDefault();
      aboutOpen = true;
      return;
    }
    if (!e.ctrlKey) return;
    if (e.key === "q" || e.key === "Q") {
      e.preventDefault();
      getCurrentWindow().close();
    } else if (e.code === "Tab") {
      e.preventDefault();
      const dir = e.shiftKey ? -1 : 1;
      const idx = tabs.findIndex((t) => t.id === active);
      select(tabs[(idx + dir + tabs.length) % tabs.length].id);
    }
  }
  import WindowControls from "./components/WindowControls.svelte";
  import { frameless, maximized } from "./lib/window";
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="h-screen bg-dark {$frameless && !$maximized ? 'window-card' : ''}">
<div class="flex h-full overflow-hidden">
  <Sidebar {tabs} {active} onSelect={select} {appVersion} />

  <div class="flex-1 flex flex-col overflow-hidden min-w-0">
    {#if $frameless}
      <div class="relative h-7 shrink-0" data-tauri-drag-region>
        <div class="absolute right-2 top-[3px]"><WindowControls /></div>
      </div>
    {/if}
    <div class="p-4 pb-0 shrink-0 {$frameless ? 'pt-0' : ''}" data-tauri-drag-region="deep">
      <div
        class="relative rounded border border-border overflow-hidden flex flex-col items-start justify-end p-4 min-h-[100px]"
        style="background: linear-gradient(rgba(20, 20, 22, 0.25), rgba(20, 20, 22, 0.82)), url('/assets/hero/repo-selector-background.jpg'); background-size: cover; background-position: center;"
      >
        <p class="text-white font-semibold text-[0.95rem]">{activeTab.title}</p>
        <p class="text-[0.75rem] text-white/85 mt-0.5">{activeTab.description}</p>
      </div>
    </div>
    <div id="main-scroll" class="content-scroll flex-1 overflow-y-auto p-7">
      <svelte:component this={ActiveView} />
    </div>
  </div>
</div>

{#if aboutOpen}
  <AboutDialog onClose={() => (aboutOpen = false)} />
{/if}
</div>
