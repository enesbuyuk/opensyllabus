import { Popover as P } from 'radix-ui'
import * as React from 'react'
import { cn } from '@/lib/utils'

export const Popover = P.Root
export const PopoverTrigger = P.Trigger

export function PopoverContent({ className, align = 'end', sideOffset = 8, ...props }: React.ComponentProps<typeof P.Content>) {
  return (
    <P.Portal>
      <P.Content
        align={align}
        sideOffset={sideOffset}
        className={cn('z-50 w-80 rounded-xl border bg-popover text-popover-foreground shadow-xl outline-none data-[state=open]:animate-fade-in', className)}
        {...props}
      />
    </P.Portal>
  )
}
