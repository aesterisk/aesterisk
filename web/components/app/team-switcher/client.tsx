"use client";

import { cn, getPlan, getPrimaryChars } from "@/lib/utils";
import { UserTeam } from "@/types/team";
import { Avatar, AvatarFallback } from "@ui/avatar";
import { Button } from "@ui/button";
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandSeparator } from "@ui/command";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@ui/dialog";
import { Input } from "@ui/input";
import { Label } from "@ui/label";
import { Popover, PopoverContent, PopoverTrigger } from "@ui/popover";
import { Skeleton } from "@ui/skeleton";
import { Check, ChevronsUpDown, Plus } from "lucide-react";
import { ComponentPropsWithoutRef, useState } from "react";

type PopoverTriggerProps = ComponentPropsWithoutRef<typeof PopoverTrigger>;

interface TeamSwitcherProps extends PopoverTriggerProps {
	selectedTeam: UserTeam | null;
	personalTeam: UserTeam;
	otherTeams: UserTeam[];
	// todo: what is this no-whitespace arrow style
	//       i don't like it
	//       too tired to fix
	action: (team: string)=> Promise<void>;
}

export function TeamSwitcherInternal({ selectedTeam, personalTeam, otherTeams, className, action }: TeamSwitcherProps) {
	const [open, setOpen] = useState(false);
	const isLoading = selectedTeam === null;
	const [showNewTeamDialog, setShowNewTeamDialog] = useState(false);
	const [searchQuery, setSearchQuery] = useState("");
	const [newTeamName, setNewTeamName] = useState("");

	return (
		<Dialog open={showNewTeamDialog} onOpenChange={setShowNewTeamDialog}>
			<Popover open={open} onOpenChange={setOpen}>
				<PopoverTrigger asChild>
					<Button
						variant="ghost"
						role="combobox"
						aria-expanded={open}
						aria-label="Select a team"
						className={cn("w-min justify-between gap-2", className)}
					>
						{
							isLoading
								? (
									<Skeleton className="h-5 w-5 rounded-full" />
								)
								: (
									<Avatar className="h-5 w-5">
										{ /* todo: maybe add the option for custom team avatars */ }
										<AvatarFallback className={cn("text-[10px] font-semibold", getPlan(selectedTeam.team).color)}>{ getPrimaryChars(selectedTeam.team.name) }</AvatarFallback>
									</Avatar>
								)
						}
						{ isLoading ? <Skeleton className="h-2 w-28" /> : selectedTeam.team.name }
						<ChevronsUpDown className="ml-auto h-4 w-4 shrink-0 opacity-50" />
					</Button>
				</PopoverTrigger>
				<PopoverContent className="mr-4 p-0 w-64">
					<Command>
						<CommandList>
							<CommandInput placeholder="Search team..." onValueChange={setSearchQuery} />
							<CommandEmpty>
								{ "You weren't invited to that party." }
								<Button
									variant="outline"
									className="mt-4"
									onClick={
										() => {
											setOpen(false);
											setNewTeamName(searchQuery);
											setShowNewTeamDialog(true);
										}
									}
								>
									<Plus className="h-5 w-5" strokeWidth={1.5} />
									{ "Create new team" }
								</Button>
							</CommandEmpty>
							<CommandGroup heading="Personal">
								<CommandItem
									value={personalTeam.team.path}
									onSelect={
										async() => {
											setOpen(false);
											await action(personalTeam.team.path);
										}
									}
									className="text-sm"
								>
									<Avatar className="h-5 w-5">
										<AvatarFallback className={cn("text-[10px] font-semibold", getPlan(personalTeam.team).color)}>{ getPrimaryChars(personalTeam.team.name) }</AvatarFallback>
									</Avatar>
									{ personalTeam.team.name }
									<Check className={cn("ml-auto h-4 w-4", !isLoading && (selectedTeam.team.id === personalTeam.team.id) ? "opacity-100" : "opacity-0")} />
								</CommandItem>
							</CommandGroup>
							<CommandGroup heading="Teams">
								{
									otherTeams.map((team) => (
										<CommandItem
											key={`team-switcher-${team.team.id}`}
											value={team.team.path}
											onSelect={
												async() => {
													setOpen(false);
													await action(team.team.path);
												}
											}
											className="text-sm"
										>
											<Avatar className="h-5 w-5">
												<AvatarFallback className={cn("text-[10px] font-semibold", getPlan(team.team).color)}>{ getPrimaryChars(team.team.name) }</AvatarFallback>
											</Avatar>
											{ team.team.name }
											<Check className={cn("ml-auto h-4 w-4", !isLoading && (selectedTeam.team.id === team.team.id) ? "opacity-100" : "opacity-0")} />
										</CommandItem>
									))
								}
							</CommandGroup>
							<CommandSeparator />
							<CommandGroup>
								<DialogTrigger asChild>
									<CommandItem
										onSelect={
											() => {
												setOpen(false);
												setShowNewTeamDialog(true);
											}
										}
									>
										<Plus className="h-5 w-5" strokeWidth={1.5} />
										{ "Create new team" }
									</CommandItem>
								</DialogTrigger>
							</CommandGroup>
						</CommandList>
					</Command>
				</PopoverContent>
			</Popover>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>{ "Create team" }</DialogTitle>
					<DialogDescription>{ "Create a new team to manage your servers collaboratively" }</DialogDescription>
				</DialogHeader>
				<div className="py-2 pb-4 space-y-2">
					<Label htmlFor="new-team-name">{ "Team name" }</Label>
					<Input id="new-team-name" placeholder="Monsters Inc." value={newTeamName} onChange={(ev) => setNewTeamName(ev.target.value)} />
				</div>
				<DialogFooter>
					<Button variant="outline" onClick={() => setShowNewTeamDialog(false)}>
						{ "Cancel" }
					</Button>
					<Button type="submit" onClick={() => setShowNewTeamDialog(false)}>
						{ "Create" }
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
