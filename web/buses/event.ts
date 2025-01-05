import { createEventBus, EventMap } from "@/lib/bus";
import { ID } from "@/packets/packet";
import { socketBus } from "./socket";
import { EventOf, EventType } from "@/packets/events";

export interface EventsBus extends EventMap {
	// todo: this looks like a nice typescript type defenition to make generic ...
	[EventType.NodeStatus]: (payload: EventOf<EventType.NodeStatus>)=> void;
}

export const eventsBus = createEventBus<EventsBus>({
	onError: (e) => {
		console.error(e);
	},
	preListen: (key, _handler, preParams) => {
		socketBus.on("connected", () => {
			socketBus.emit(
				ID.WSListen,
				[
					{
						// ... which would make this perfect
						event: key as EventType,
						daemons: preParams,
					},
				],
			);
		});

		return false;
	},
});
