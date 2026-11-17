<script lang="ts">
  import type { Snippet } from 'svelte';
  import Skeleton from './Skeleton.svelte';

  interface Column {
    key: string;
    label: string;
    numeric?: boolean;
    align?: 'left' | 'right';
    mono?: boolean;
    width?: string;
    render?: Snippet<[Record<string, unknown>]>;
  }

  interface Props {
    columns: Column[];
    rows: Record<string, unknown>[];
    caption?: string;
    rowKey?: (row: Record<string, unknown>) => string;
    rowHref?: (row: Record<string, unknown>) => string;
    loading?: boolean;
    empty?: Snippet;
    selectedKey?: string;
    onrowclick?: (row: Record<string, unknown>) => void;
  }

  let {
    columns,
    rows,
    caption = 'Tabel',
    rowKey,
    rowHref,
    loading = false,
    empty,
    selectedKey,
    onrowclick
  }: Props = $props();

  function isNumeric(column: Column) {
    return column.numeric || column.align === 'right' || column.mono === true;
  }

  function cellText(row: Record<string, unknown>, column: Column) {
    const value = row[column.key];
    return value === null || value === undefined ? '' : String(value);
  }
</script>

<div class="table-wrap">
  <table>
    <caption class="sr-only">{caption}</caption>
    <thead>
      <tr>
        {#each columns as column (column.key)}
          <th
            scope="col"
            class:num={isNumeric(column)}
            style={column.width ? `width: ${column.width}` : undefined}
          >
            {column.label}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody aria-busy={loading || undefined}>
      {#if loading}
        {#each Array(8) as _, i (i)}
          <tr><td colspan={columns.length}><Skeleton shape="row" height="38px" /></td></tr>
        {/each}
      {:else if rows.length === 0}
        <tr><td colspan={columns.length}>{@render empty?.()}</td></tr>
      {:else}
        {#each rows as row, i (rowKey?.(row) ?? i)}
          <tr
            class:selected={selectedKey !== undefined && rowKey?.(row) === selectedKey}
            class:clickable={Boolean(onrowclick)}
            onclick={onrowclick ? () => onrowclick(row) : undefined}
          >
            {#each columns as column, colIndex (column.key)}
              <td class:num={isNumeric(column)}>
                {#if colIndex === 0 && rowHref}
                  <a href={rowHref(row)}>{#if column.render}{@render column.render(row)}{:else}{cellText(row, column)}{/if}</a>
                {:else if column.render}
                  {@render column.render(row)}
                {:else}
                  {cellText(row, column)}
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
</div>

<style>
  .table-wrap {
    width: 100%;
    overflow-x: auto;
    background: var(--surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-card);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    line-height: 20px;
  }

  th {
    height: 38px;
    padding: 0 12px;
    border-bottom: 1px solid var(--hairline);
    color: var(--ink-10);
    font-size: 12px;
    line-height: 16px;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
  }

  td {
    height: 38px;
    padding: 0 12px;
    border-bottom: 1px solid var(--hairline);
    color: var(--ink-11);
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  td:first-child {
    color: var(--ink-12);
  }

  th.num,
  td.num {
    text-align: right;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  tbody tr:hover td {
    background: var(--hover);
  }

  tbody tr.selected td {
    background: var(--attention-tint);
    box-shadow: inset 2px 0 0 var(--attention);
  }

  tbody tr.clickable {
    cursor: pointer;
  }

  a {
    color: inherit;
    text-decoration: none;
  }

  a:hover {
    text-decoration: underline;
  }

  @media (max-width: 767px) {
    td,
    th {
      max-width: 220px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }
</style>
