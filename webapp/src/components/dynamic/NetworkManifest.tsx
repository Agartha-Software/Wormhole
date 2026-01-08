import type { Network } from "../../actions/get/networks";
import Status from "./Status";

export default function ({network}: {network: Network}) {
    return (
        <div className="flex justify-between items-center w-full bg-red-200">
            <p>{network.name}</p>
            <Status status={network.status} />
        </div>
    )
}