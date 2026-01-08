export type NetworkStatus = "online" | "offline" | "trouble";

export function status_tailwind_color(status: NetworkStatus | undefined) {
    switch (status) {
        case "online":
            return "bg-green-400";
        case "trouble":
            return "bg-amber-500";
        case "offline":
            return "bg-red-500";
        case undefined:
            return "bg-gray-500";
    }
}