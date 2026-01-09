/**
 * Classname utility function (from existing utils.ts in admin/web)
 * Merged from clsx and tailwind-merge
 */

import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
