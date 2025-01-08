"use client";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { ReactNode, useRef } from "react";
import { toast } from "sonner";

export default function CodeBlock({ children, className, copyString }: {
	children: ReactNode;
	className?: string;
	copyString?: string;
}) {
	const ref = useRef<HTMLPreElement>(null);

	const copy = () => {
		if(!ref.current) return;
		if(!ref.current.textContent) return;
		window.navigator.clipboard.writeText(copyString ?? ref.current.textContent);
		toast.success("Copied to clipboard", {
			action: {
				label: "Dismiss",
				onClick: () => {},
			},
		});
	};

	return (
		<TooltipProvider>
			<Tooltip>
				<TooltipTrigger asChild>
					<Button variant="outline" className={cn("my-2 justify-start h-max text-left", className)} type="button">
						<pre className="rounded-md cursor-pointer font-mono" onClick={copy} ref={ref}>
							{ children }
						</pre>
					</Button>
				</TooltipTrigger>
				<TooltipContent>
					<span>{ "Copy to Clipboard" }</span>
				</TooltipContent>
			</Tooltip>
		</TooltipProvider>
	);
}
