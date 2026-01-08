import { status_tailwind_color } from "../../ts/NetworkStatus";

export type StatusType = "online" | "offline" | "trouble" | undefined;

export default function ({status}: {status: StatusType}) {
    return <div class={"rounded-full aspect-square h-full min-h-4 border-2 border-black " + status_tailwind_color(status)}/>
}