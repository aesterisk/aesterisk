import type { Metadata, Viewport } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import React from "react";
import { dev } from "@/lib/dev";

const geistSans = Geist({
	variable: "--font-geist-sans",
	subsets: ["latin"],
});

const geistMono = Geist_Mono({
	variable: "--font-geist-mono",
	subsets: ["latin"],
});

export const metadata: Metadata = {
	// todo: add more metadata
	title: "Aesterisk",
	description: "Server management done right.",
	applicationName: "Aesterisk",
	openGraph: {
		type: "website",
	},
	robots: {
		index: true,
		follow: true,
	},
};

export const viewport: Viewport = {
	themeColor: [
		{
			media: "(prefers-color-scheme: light)",
			color: "hsl(210, 25%, 98.4%)",
		},
		{
			// todo: dark mode for real
			media: "(prefers-color-scheme: dark)",
			color: "#000000",
		},
	],
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode; }>) {
	return (
		<html lang="en">
			{
				dev() && (
					<head>
						{ /* eslint-disable-next-line @next/next/no-sync-scripts */ }
						<script src="http://localhost:8097" />
					</head>
				)
			}
			<body className={`${geistSans.variable} ${geistMono.variable} font-sans antialiased w-screen h-screen overflow-hidden`}>
				{ children }
			</body>
		</html>
	);
}
