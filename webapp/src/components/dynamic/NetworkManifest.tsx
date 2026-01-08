import type { Network } from "../../actions/get/networks";
import Status from "./Status";

export default function ({network, selected}: {network: Network, selected: boolean}) {
    return (
        <a href={`/network/${network.name}`} className={`flex items-center gap-2.5 py-2 px-5 hover:bg-slate-500 rounded-lg ${selected ? "bg-slate-500" : ""}`}>
            <Status status={network.status} />
            <p>{network.name}</p>
        </a>
    )
}