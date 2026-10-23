import { Tooltip as P } from 'radix-ui'
import * as React from 'react'
import { cn } from '@/lib/utils'

export const TooltipProvider = P.Provider

export function Tooltip({ content, children, side = 'top' }: { content: React.ReactNode; children: React.ReactNode; side?: 'top' | 'right' | 'bottom' | 'left' }) {
  return (
    <P.Root>
      <P.Trigger asChild>{children}</P.Trigger>
      <P.Portal>
        <P.Content
          side={side}
          sideOffset={6}
          className={cn('z-50 rounded-md bg-foreground px-2 py-1 text-xs text-background shadow-md data-[state=delayed-open]:animate-fade-in')}
        >
          {content}
        </P.Content>
      </P.Portal>
    </P.Root>
  )
}
