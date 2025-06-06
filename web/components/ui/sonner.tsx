"use client";

import { Loader2 } from "lucide-react";
import { useTheme } from "next-themes";
import { CSSProperties } from "react";
import { Toaster as Sonner, ToasterProps } from "sonner";

const Toaster = ({ ...props }: ToasterProps) => {
	const { theme = "system" } = useTheme();

	// todo: dark mode support someday

	return (
		<Sonner
			// theme={theme as ToasterProps["theme"]}
			theme="light"
			className="toaster group"
			style={
				{
					"--normal-bg": "var(--popover)",
					"--normal-text": "var(--popover-foreground)",
					"--normal-border": "var(--border)",
				} as CSSProperties
			}
			icons={
				{
					loading: <Loader2 className="size-4 animate-spin" />,
				}
			}
			{...props}
		/>
	);
};

export { Toaster };
