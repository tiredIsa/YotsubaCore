export type ClassValue = string | null | undefined | false;

export const cn = (...inputs: ClassValue[]) => inputs.filter(Boolean).join(" ");