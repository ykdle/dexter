<script lang="ts">
    export let difficulty: string | undefined;
    export let hover: boolean = false;

</script>
{#if difficulty === "노말"}
    <span>[노말]</span>
{:else if difficulty === "하드"}
    <span class="text-yellow-300 {hover ? 'group-hover:text-accent-500' : ''}">[{difficulty}]</span>
{:else if difficulty === "헬" || difficulty === "도전" || difficulty === "시련"}
    <span class="text-amber-600 {hover ? 'group-hover:text-accent-500' : ''}">[{difficulty}]</span>
{:else if difficulty === "솔로"}
    <span class="text-cyan-400 {hover ? 'group-hover:text-accent-500' : ''}">[솔로]</span>
{:else if difficulty === "Extreme" || difficulty === "The First"}
    <span class="text-purple-500 {hover ? 'group-hover:text-accent-500' : ''}">[{difficulty}]</span>
{:else}
    &nbsp;
{/if}