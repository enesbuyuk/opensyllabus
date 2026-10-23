import { Dialog as SheetPrimitive } from 'radix-ui'
import * as React from 'react'
import { cn } from '@/lib/utils'

export const Sheet = SheetPrimitive.Root

export function SheetContent({ className, children, side = 'left', ...props }: React.ComponentProps<typeof SheetPrimitive.Content> & { side?: 'left' | 'right' }) {
  return (
    <SheetPrimitive.Portal>
      <SheetPrimitive.Overlay className="fixed inset-0 z-50 bg-black/60 data-[state=open]:animate-fade-in" />
      <SheetPrimitive.Content
        className={cn(
          'fixed inset-y-0 z-50 flex w-72 max-w-[85vw] flex-col bg-sidebar shadow-2xl data-[state=open]:animate-fade-in',
          side === 'left' ? 'left-0 border-r' : 'right-0 border-l',
          className,
        )}
        {...props}
      >
        <SheetPrimitive.Title className="sr-only">Navigation</SheetPrimitive.Title>
        <SheetPrimitive.Description className="sr-only">Main navigation menu</SheetPrimitive.Description>
        {children}
      </SheetPrimitive.Content>
    </SheetPrimitive.Portal>
  )
}
