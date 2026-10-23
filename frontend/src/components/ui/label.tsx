import { Label as LabelPrimitive } from 'radix-ui'
import * as React from 'react'
import { cn } from '@/lib/utils'

export function Label({ className, ...props }: React.ComponentProps<typeof LabelPrimitive.Root>) {
  return (
    <LabelPrimitive.Root
      className={cn('flex items-center gap-2 text-[13px] font-medium leading-none select-none peer-disabled:opacity-50', className)}
      {...props}
    />
  )
}
