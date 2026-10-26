import { z } from 'zod'

/** Mirrors the server's password policy (see backend auth::password). */
export const passwordSchema = z
  .string()
  .min(10, 'Use at least 10 characters')
  .max(128, 'Use at most 128 characters')
  .regex(/[^A-Za-z]/, 'Include at least one number or symbol')

/** Optional login name, mirrors backend util::is_valid_username. Empty means none. */
export const usernameSchema = z
  .string()
  .trim()
  .toLowerCase()
  .refine((v) => v === '' || /^[a-z0-9][a-z0-9._-]{2,31}$/.test(v), 'Use 3-32 characters: letters, digits, ".", "_" or "-", starting with a letter or digit')
