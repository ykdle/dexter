<script lang="ts">
    import { abbreviateNumber } from "$lib/utils/numbers";
    import { settings } from "$lib/utils/settings";
    import { takingScreenshot } from "$lib/utils/stores";
    import { getVersion } from "@tauri-apps/api/app";
    import DifficultyLabel from "../shared/DifficultyLabel.svelte";
    import BossOnlyDamage from "../shared/BossOnlyDamage.svelte";

    export let bossName: string;
    export let difficulty: string | undefined;
    export let date: string;
    export let encounterDuration: string;
    export let totalDamageDealt: number;
    export let dps: number;
    export let cleared: boolean;
    export let bossOnlyDamage: boolean;
    export let raidGate: string | undefined;
</script>

{#if $takingScreenshot}
    <div class="flex items-center justify-between px-1 tracking-tight">
        <div>
            {#if cleared}
                <span class="text-lime-400">[Cleared]</span>
            {/if}
            {#if bossOnlyDamage}
                <BossOnlyDamage/>
            {/if}
            <span class="font-medium">
                {#if $settings.general.showDifficulty && difficulty}
                    <DifficultyLabel {difficulty} />
                    {#if $settings.general.showGate && raidGate}
                        <span class="text-sky-200">[{raidGate}]</span>
                    {/if}
                    {bossName}
                {:else}
                    {#if $settings.general.showGate && raidGate}
                        <span class="text-sky-200">[{raidGate}]</span>
                    {/if}
                    {bossName}
                {/if}
            </span><span class="ml-2 font-mono text-xs">{date}</span>
        </div>
        {#await getVersion() then version}
            {#if !$settings.general.hideLogo}
                <div class="">
                    LOA Logs KR v{version}
                </div>
            {:else}
                <div class="font-mono text-xs">
                    v{version}
                </div>
            {/if}
        {/await}
    </div>
{/if}
<div class="px-1 text-sm" class:pb-2={$takingScreenshot} id="header">
    <div class="flex items-center justify-between">
        <div class="flex space-x-2">
            <div>
                {encounterDuration}
            </div>
            <div class="flex space-x-1 tracking-tighter text-gray-300">
                <div>총 데미지:</div>
                {#if $settings.logs.abbreviateHeader}
                    <div class="text-white">
                        {abbreviateNumber(totalDamageDealt)}
                    </div>
                {:else}
                    <div class="text-white">
                        {totalDamageDealt.toLocaleString()}
                    </div>
                {/if}
            </div>
            <div class="flex space-x-1 tracking-tighter text-gray-300">
                <div>총 DPS:</div>
                {#if $settings.logs.abbreviateHeader}
                    <div class="text-white">
                        {abbreviateNumber(dps)}
                    </div>
                {:else}
                    <div class="text-white">
                        {dps.toLocaleString(undefined, {
                            minimumFractionDigits: 0,
                            maximumFractionDigits: 0
                        })}
                    </div>
                {/if}
            </div>
        </div>
        {#if $takingScreenshot && !$settings.general.hideLogo}
            <div class="font-mono text-xs">
                {"github.com/MaccolSolZico/loa-logs-kr"}
            </div>
        {/if}
    </div>
</div>
