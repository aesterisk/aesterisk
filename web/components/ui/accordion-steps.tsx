"use client";

import * as React from "react";
import * as AccordionPrimitive from "@radix-ui/react-accordion";

import { cn } from "@/lib/utils";

const StepAccordion = React.forwardRef<
	React.ElementRef<typeof AccordionPrimitive.Root>,
	React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Root>
>(({ className, ...props }, ref) => (
	<AccordionPrimitive.Root
		ref={ref}
		className={cn("border rounded-lg bg-muted-40", className)}
		{...props}
	/>
));
StepAccordion.displayName = "AccordionItem";

const StepAccordionItem = React.forwardRef<
	React.ElementRef<typeof AccordionPrimitive.Item>,
	React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Item> & {
		last?: boolean;
	}
>(({ className, last, ...props }, ref) => (
	<AccordionPrimitive.Item
		ref={ref}
		className={cn(last || "border-b", className)}
		{...props}
	/>
));
StepAccordionItem.displayName = "AccordionItem";

const StepAccordionTrigger = React.forwardRef<
	React.ElementRef<typeof AccordionPrimitive.Trigger>,
	React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Trigger> & {
		state: "done" | "pending" | "not-started";
	}
>(({ className, children, state, ...props }, ref) => (
	<AccordionPrimitive.Header className="flex">
		<AccordionPrimitive.Trigger
			ref={ref}
			className={
				cn(
					"flex flex-1 items-center [&[data-state=open]]:border-b gap-4 p-4 font-medium transition-all",
					className,
				)
			}
			{...props}
		>
			<div className={cn("w-2 h-2 rounded-full indicator bg-rose-500", state === "done" && "bg-emerald-500", state === "pending" && "bg-yellow-300")} />
			{ children }
		</AccordionPrimitive.Trigger>
	</AccordionPrimitive.Header>
));
StepAccordionTrigger.displayName = AccordionPrimitive.Header.displayName;

const StepAccordionContent = React.forwardRef<
	React.ElementRef<typeof AccordionPrimitive.Content>,
	React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Content>
>(({ className, children, ...props }, ref) => (
	<AccordionPrimitive.Content
		ref={ref}
		className="bg-background rounded-lg overflow-hidden text-sm transition-all data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down"
		{...props}
	>
		<div className={cn("p-4", className)}>{ children }</div>
	</AccordionPrimitive.Content>
));
StepAccordionContent.displayName = AccordionPrimitive.Content.displayName;

export { StepAccordion, StepAccordionItem, StepAccordionTrigger as StepAccordionHeader, StepAccordionContent };
