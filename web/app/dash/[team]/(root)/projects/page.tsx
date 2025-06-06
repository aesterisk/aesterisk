export default function Projects({ params }: Readonly<{ params: Promise<{ team: string; }>; }>) {
	const team = params.then((p) => p.team);

	return (
		<main className="px-4 py-5 w-full">
			<span>{ "Projects" }</span>
		</main>
	);
}
