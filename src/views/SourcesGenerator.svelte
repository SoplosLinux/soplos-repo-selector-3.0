<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { _ } from "svelte-i18n";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Gauge, FileCog, Loader2, CheckCircle2 } from "lucide-svelte";
  import { api } from "../lib/api";
  import { sortMirrors, KNOWN_DISTROS, KNOWN_COMPONENTS, type MirrorResult } from "../lib/speedtest";
  import { detectSystemState } from "../lib/detectState";
  import type { Repo } from "../lib/repos";
  import Modal from "../components/Modal.svelte";

  const DEFAULT_MIRROR = "http://deb.debian.org/debian";
  const TERMINAL_STATUSES = new Set(["success", "connection_failed", "no_valid_files", "error", "timeout"]);

  let selectedDistros = new Set<string>(["trixie"]);
  let selectedComponents = new Set<string>(["main"]);
  let backports = false;
  let debSrc = false;
  let mirror = DEFAULT_MIRROR;
  let detected = false;

  let testing = false;
  let mirrorCount = 0;
  let testedCount = 0;
  // Matches 2.x's _on_single_result/_on_speed_test_finished: one list,
  // rows appear in the order each mirror is first touched and update in
  // place (own progress bar, own live status) — never reordered while the
  // scan runs. Only once the whole scan finishes does it get rebuilt in
  // sorted order (see sortMirrors / findFastestMirror below).
  let rowOrder: string[] = [];
  let resultsByUrl: Map<string, MirrorResult> = new Map();
  let unlisten: UnlistenFn | null = null;

  let generating = false;
  let errorMessage: string | null = null;
  let successMessage = false;

  onMount(async () => {
    try {
      mirrorCount = await api.invoke<number>("get_mirror_count");
    } catch (error) {
      console.error("Failed to read mirror count", error);
    }

    try {
      const repos = await api.invoke<Repo[]>("list_repos");
      const state = detectSystemState(repos);
      if (state.distros.size > 0) {
        selectedDistros = state.distros;
        detected = true;
      }
      if (state.components.size > 0) selectedComponents = state.components;
      backports = state.backports;
      debSrc = state.debSrc;
      if (state.mirror) mirror = state.mirror;
    } catch (error) {
      console.error("Failed to detect existing repository state", error);
    }

    unlisten = await listen<MirrorResult>("speedtest-progress", (e) => {
      if (!resultsByUrl.has(e.payload.url)) {
        rowOrder = [...rowOrder, e.payload.url];
      }
      const prev = resultsByUrl.get(e.payload.url);
      const wasTerminal = prev && TERMINAL_STATUSES.has(prev.status);
      const isNowTerminal = TERMINAL_STATUSES.has(e.payload.status);

      resultsByUrl.set(e.payload.url, e.payload);
      resultsByUrl = resultsByUrl;

      if (isNowTerminal && !wasTerminal) {
        testedCount += 1;
      }
    });
  });

  onDestroy(() => {
    unlisten?.();
  });

  // While testing: insertion order, each row showing its own live state.
  // Once finished: 2.x's own final order (success by speed desc, then
  // failures) — see sortMirrors in lib/speedtest.ts.
  $: displayedResults = testing ? rowOrder.map((url) => resultsByUrl.get(url)!).filter(Boolean) : sortMirrors([...resultsByUrl.values()]);
  $: progressPct = mirrorCount > 0 ? Math.min(100, Math.round((testedCount / mirrorCount) * 100)) : 0;

  function toggle(set: Set<string>, value: string) {
    if (set.has(value)) set.delete(value);
    else set.add(value);
    detected = false;
    return new Set(set);
  }

  async function findFastestMirror() {
    testing = true;
    testedCount = 0;
    rowOrder = [];
    resultsByUrl = new Map();
    try {
      const final = await api.invoke<MirrorResult[]>("test_mirrors");
      const m = new Map<string, MirrorResult>();
      for (const r of final) m.set(r.url, r);
      resultsByUrl = m;
      mirrorCount = final.length;
      testedCount = final.length;
      const best = sortMirrors([...m.values()])[0];
      if (best && best.status === "success") mirror = best.url;
    } catch (error) {
      console.error("Speed test failed", error);
      errorMessage = String(error);
    } finally {
      testing = false;
    }
  }

  async function generate() {
    if (selectedDistros.size === 0 || selectedComponents.size === 0) return;
    generating = true;
    try {
      await api.invoke("generate_sources", {
        input: {
          distros: [...selectedDistros],
          components: [...selectedComponents],
          backports,
          debSrc,
          mirror,
        },
      });
      successMessage = true;
    } catch (error) {
      console.error("Failed to generate sources", error);
      errorMessage = String(error);
    } finally {
      generating = false;
    }
  }
</script>

<div class="flex flex-col gap-6">
  {#if detected}
    <p class="text-[0.76rem] text-primary">{$_("sourcesGenerator.detected")}</p>
  {/if}

  <section class="border border-border rounded-lg p-4">
    <h2 class="text-[0.85rem] font-semibold text-text-bright">{$_("sourcesGenerator.distributions")}</h2>
    <p class="text-[0.74rem] text-text-muted mb-2">{$_("sourcesGenerator.distributionsHint")}</p>
    <div class="flex flex-wrap gap-2">
      {#each KNOWN_DISTROS as distro}
        <button
          class="px-3 py-1.5 rounded border text-[0.78rem] {selectedDistros.has(distro) ? 'bg-primary/15 border-primary text-primary font-semibold' : 'border-border text-text hover:bg-surface'}"
          on:click={() => (selectedDistros = toggle(selectedDistros, distro))}
        >
          {distro}
        </button>
      {/each}
    </div>
    <button class="flex items-center gap-2 mt-3 text-[0.8rem] text-text" on:click={() => (backports = !backports)}>
      <span class="relative w-[32px] h-[18px] rounded-full transition-colors shrink-0 {backports ? 'bg-primary' : 'bg-surface-2'}" aria-pressed={backports}>
        <span class="absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-text-bright transition-transform {backports ? 'translate-x-[14px]' : ''}"></span>
      </span>
      {$_("sourcesGenerator.includeBackports")}
    </button>
  </section>

  <section class="border border-border rounded-lg p-4">
    <h2 class="text-[0.85rem] font-semibold text-text-bright">{$_("sourcesGenerator.components")}</h2>
    <p class="text-[0.74rem] text-text-muted mb-2">{$_("sourcesGenerator.componentsHint")}</p>
    <div class="flex flex-wrap gap-2">
      {#each KNOWN_COMPONENTS as component}
        <button
          class="px-3 py-1.5 rounded border text-[0.78rem] {selectedComponents.has(component) ? 'bg-primary/15 border-primary text-primary font-semibold' : 'border-border text-text hover:bg-surface'}"
          on:click={() => (selectedComponents = toggle(selectedComponents, component))}
        >
          {component}
        </button>
      {/each}
    </div>
    <button class="flex items-center gap-2 mt-3 text-[0.8rem] text-text" on:click={() => (debSrc = !debSrc)}>
      <span class="relative w-[32px] h-[18px] rounded-full transition-colors shrink-0 {debSrc ? 'bg-primary' : 'bg-surface-2'}" aria-pressed={debSrc}>
        <span class="absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-text-bright transition-transform {debSrc ? 'translate-x-[14px]' : ''}"></span>
      </span>
      {$_("sourcesGenerator.includeSourcePackages")}
    </button>
  </section>

  <section class="border border-border rounded-lg p-4">
    <div class="flex items-center justify-between">
      <h2 class="text-[0.85rem] font-semibold text-text-bright">{$_("sourcesGenerator.mirror")}</h2>
      <button
        class="flex items-center gap-1.5 px-3 py-1.5 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.78rem] disabled:opacity-40"
        disabled={testing}
        on:click={findFastestMirror}
      >
        {#if testing}
          <Loader2 size={14} class="animate-spin" />
        {:else}
          <Gauge size={14} />
        {/if}
        {testing ? $_("sourcesGenerator.testing") : $_("sourcesGenerator.findFastest")}
      </button>
    </div>
    <p class="text-[0.74rem] text-text-muted">{$_("sourcesGenerator.mirrorHint")}</p>
    <p class="text-[0.78rem] text-text-muted mb-2 mt-1 break-all">{$_("sourcesGenerator.currentMirror")}: <span class="text-text-bright">{mirror}</span></p>

    {#if testing}
      <div class="mb-3">
        <div class="h-1.5 rounded-full bg-surface-2 overflow-hidden">
          <div class="h-full bg-primary transition-[width]" style="width: {progressPct}%"></div>
        </div>
        <p class="text-[0.7rem] text-text-muted mt-1">{$_("sourcesGenerator.progress", { values: { tested: testedCount, total: mirrorCount } })}</p>
      </div>
    {/if}

    {#if displayedResults.length > 0}
      <div class="flex flex-col gap-1.5 max-h-[320px] overflow-y-auto content-scroll">
        {#each displayedResults as r, i (r.url)}
          {@const isTerminal = TERMINAL_STATUSES.has(r.status)}
          <div class="px-3 py-2 rounded border {mirror === r.url ? 'border-primary bg-primary/10' : 'border-border'}">
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-1.5 min-w-0">
                {#if !testing && i === 0 && r.status === "success"}
                  <span class="text-[0.62rem] font-semibold text-primary bg-primary/15 rounded px-1.5 py-0.5 shrink-0">{$_("sourcesGenerator.recommended")}</span>
                {/if}
                <span class="min-w-0 flex-1 truncate text-text-bright text-[0.78rem]">{r.url}</span>
              </div>
              {#if !isTerminal}
                <span class="text-[0.7rem] text-text-muted shrink-0">
                  {#if r.status === "measuring_latency"}
                    {$_("sourcesGenerator.measuringLatency")}
                  {:else if r.status === "downloading"}
                    {r.speedMbps.toFixed(1)} MB/s
                  {:else}
                    {$_("sourcesGenerator.statusTesting")}
                  {/if}
                </span>
              {:else if mirror === r.url}
                <span class="flex items-center gap-1 text-[0.72rem] text-primary font-semibold shrink-0"><CheckCircle2 size={14} />{$_("sourcesGenerator.selected")}</span>
              {:else if r.status === "success"}
                <button class="px-2.5 py-1.5 rounded border border-border hover:border-primary hover:text-primary text-[0.72rem] font-semibold shrink-0" on:click={() => (mirror = r.url)}>
                  {$_("sourcesGenerator.use")}
                </button>
              {/if}
            </div>
            <div class="text-[0.7rem] text-text-muted mt-0.5">
              {r.country}
              {#if r.status === "success"}
                {"  ·  "}{#if r.speedMbps > 0}{r.speedMbps.toFixed(1)} MB/s · {/if}{r.latencyMs.toFixed(0)} ms
              {:else if isTerminal}
                {"  ·  "}<span class="text-danger">{$_("sourcesGenerator.statusFailed")}</span>
              {/if}
            </div>
            {#if !isTerminal}
              <div class="h-1 rounded-full bg-surface-2 overflow-hidden mt-1">
                <div class="h-full bg-primary transition-[width]" style="width: {Math.round(r.progress * 100)}%"></div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <button
    class="self-start flex items-center gap-1.5 px-4 py-2.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.82rem] font-semibold disabled:opacity-40 disabled:cursor-not-allowed"
    disabled={generating || selectedDistros.size === 0 || selectedComponents.size === 0}
    on:click={generate}
  >
    <FileCog size={15} />
    {generating ? $_("sourcesGenerator.generating") : $_("sourcesGenerator.generate")}
  </button>
</div>

{#if successMessage}
  <Modal variant="info" title={$_("sourcesGenerator.successTitle")} onClose={() => (successMessage = false)}>
    <p>{$_("sourcesGenerator.successBody")}</p>
  </Modal>
{/if}

{#if errorMessage}
  <Modal variant="info" title={$_("common.errorTitle")} onClose={() => (errorMessage = null)}>
    <p>{errorMessage}</p>
  </Modal>
{/if}
