const OVERWRITE = null;

export const dev = () => OVERWRITE ?? (process.env.NODE_ENV === "development" || process.env.NEXT_PUBLIC_ENV === "development");
