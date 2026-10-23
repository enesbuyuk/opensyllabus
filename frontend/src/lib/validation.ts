import { z } from 'zod'

/** Mirrors the server's password policy (see backend auth::password). */
export const passwordSchema = z
  .string()
  .min(10, 'Use at least 10 characters')
  .max(128, 'Use at most 128 characters')
  .regex(/[^A-Za-z]/, 'Include at least one number or symbol')
