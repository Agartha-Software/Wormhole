import { useEffect, useState } from "preact/hooks";
import type { Network } from "../../actions/get/networks";
import List from "../div/List";
import { actions } from "astro:actions";
import NetworkManifest from "./NetworkManifest";

export default function () {
    const [nwList, setNwList] = useState<Network[] | undefined | "error">(undefined);
    useEffect(() => {
        actions.get_networks(null).then(nws => {
            setNwList(nws.error ? "error" : nws.data)
            if (nws.error) console.error(nws.error);
        })
    }, []);

    if (nwList === undefined) {
        return <List><p>loading...</p></List>
    } else if (nwList === "error") {
        return <List><p>Error loading networks</p></List>
    } else {
        return <List className="max-w-lg grid gap-2.5">{nwList.map(nw => <NetworkManifest network={nw} />)}</List>
    }
}