"use client";

import { useEffect, useRef } from "react";
import { toast } from "sonner";

export default function Client({ mfaEnabled }: { mfaEnabled: boolean | null; }) {
	const toasted = useRef(false);

	useEffect(() => {
		if(mfaEnabled === null) return;

		if(!toasted.current && !mfaEnabled) {
			toasted.current = true;
			setTimeout(() => {
				toast.warning("MFA is not enabled", {
					description: "Please secure your account with MFA",
					duration: Infinity,
					action: {
						label: "Settings",
						onClick: () => {
							window.open("https://github.com/settings/security#two-factor-authentication", "_blank noopener noreferrer");
						},
					},
				});
			});
		}
	}, [mfaEnabled]);

	return <></>;
}
