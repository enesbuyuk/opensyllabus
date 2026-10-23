// Themed Recharts wrappers. Colors come from CSS tokens validated for both themes
// (chart-1 / chart-2 pass CVD + contrast checks); categorical order is fixed.
import { Bar, BarChart, CartesianGrid, Legend, ResponsiveContainer, Tooltip, XAxis, YAxis, type TooltipContentProps } from 'recharts'
import { cn } from '@/lib/utils'

export const SERIES_COLORS = ['var(--chart-1)', 'var(--chart-2)'] as const

const axisProps = {
  stroke: 'var(--muted-foreground)',
  fontSize: 11,
  tickLine: false,
  axisLine: false,
} as const

function ChartTooltip({ active, payload, label, unit }: TooltipContentProps<number, string> & { unit?: string }) {
  if (!active || !payload?.length) return null
  return (
    <div className="rounded-lg border bg-popover px-3 py-2 text-xs shadow-lg">
      <div className="mb-1 font-medium text-foreground">{label}</div>
      {payload.map((p) => (
        <div key={String(p.dataKey)} className="flex items-center gap-2 text-muted-foreground">
          <span className="size-2 rounded-full" style={{ background: p.color }} aria-hidden />
          <span>{p.name}</span>
          <span className="ml-auto pl-4 font-medium text-foreground tabular-nums">
            {p.value}
            {unit}
          </span>
        </div>
      ))}
    </div>
  )
}

export interface BarSeries {
  key: string
  name: string
}

/**
 * Vertical bar chart. One series → no legend (the card title names it);
 * two series → legend always shown. A single y-axis only.
 */
export function SimpleBarChart({
  data,
  xKey,
  series,
  unit,
  height = 220,
  max,
  className,
  ariaLabel,
}: {
  data: Record<string, string | number>[]
  xKey: string
  series: BarSeries[]
  unit?: string
  height?: number
  max?: number
  className?: string
  ariaLabel: string
}) {
  return (
    <div className={cn('w-full', className)} style={{ height }} role="img" aria-label={ariaLabel}>
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data} margin={{ top: 8, right: 4, left: -16, bottom: 0 }} barGap={2} barCategoryGap="28%">
          <CartesianGrid vertical={false} stroke="var(--border)" strokeDasharray="2 4" />
          <XAxis dataKey={xKey} {...axisProps} interval="preserveStartEnd" minTickGap={8} />
          <YAxis {...axisProps} allowDecimals={false} domain={[0, max ?? 'auto']} width={44} />
          <Tooltip cursor={{ fill: 'var(--muted)', opacity: 0.5 }} content={(p) => <ChartTooltip {...(p as TooltipContentProps<number, string>)} unit={unit} />} />
          {series.length > 1 && <Legend iconType="circle" iconSize={8} wrapperStyle={{ fontSize: 12, color: 'var(--muted-foreground)', paddingTop: 8 }} />}
          {series.map((s, i) => (
            <Bar key={s.key} dataKey={s.key} name={s.name} fill={SERIES_COLORS[i]} radius={[4, 4, 0, 0]} maxBarSize={36} stroke="var(--card)" strokeWidth={1} />
          ))}
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}
