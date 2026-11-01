import { Tabs as P } from 'radix-ui'
import * as React from 'react'
import { cn } from '@/lib/utils'

export const Tabs = P.Root

type TabsVariant = 'underline' | 'segmented'
const VariantContext = React.createContext<TabsVariant>('underline')

/** `underline` for page/section tabs (design system default); `segmented` for compact view toggles. */
export function TabsList({ className, variant = 'underline', ...props }: React.ComponentProps<typeof P.List> & { variant?: TabsVariant }) {
  return (
    <VariantContext.Provider value={variant}>
      <P.List
        className={cn(
          variant === 'underline'
            ? 'inline-flex h-10 max-w-full items-stretch gap-1 overflow-x-auto overflow-y-hidden text-muted-foreground shadow-[inset_0_-1px_0_var(--border)] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden'
            : 'inline-flex h-9 items-center gap-1 rounded-md border bg-muted/60 p-1 text-muted-foreground',
          className,
        )}
        {...props}
      />
    </VariantContext.Provider>
  )
}

export function TabsTrigger({ className, ...props }: React.ComponentProps<typeof P.Trigger>) {
  const variant = React.useContext(VariantContext)
  return (
    <P.Trigger
      className={cn(
        'inline-flex items-center justify-center gap-1.5 font-medium whitespace-nowrap transition outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50 [&_svg]:size-3.5',
        variant === 'underline'
          ? 'shrink-0 border-b-2 border-transparent px-3 text-sm data-[state=active]:border-primary data-[state=active]:font-semibold data-[state=active]:text-foreground'
          : 'h-7 rounded-sm px-3 text-[13px] data-[state=active]:bg-card data-[state=active]:text-foreground data-[state=active]:shadow-xs',
        className,
      )}
      {...props}
    />
  )
}

export function TabsContent({ className, ...props }: React.ComponentProps<typeof P.Content>) {
  return <P.Content className={cn('outline-none', className)} {...props} />
}
