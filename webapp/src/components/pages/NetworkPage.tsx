import { actions } from "astro:actions";
import { useState, useEffect } from "preact/hooks";
import type { Network } from "../../actions/get/networks";
import H2 from "../text/H2";
import Status from "../dynamic/Status";

export default function ({ name }: { name: string }) {
    const [network, setNetwork] = useState<Network | null | undefined>(null);
    useEffect(() => {
        actions.get_network(name).then(nws => {
            setNetwork(nws.error ? undefined : nws.data)
            if (nws.error) console.error(nws.error);
        })
    }, []);

    return (
        <div class="flex items-center gap-5 h-6">
            <Status status={network?.status} />
            <H2>{name}</H2>
        </div>
    )
}