import { Button } from "@/components/ui/button";
import Link from "next/link";

export default function Home() {
	return (
		<main>
			{ /* todo: go to last */ }
			<Link href="/dash/personal">
				<Button variant="link">
					{ "Go to panel" }
				</Button>
			</Link>
		</main>
	);
}
