import { actions } from "astro:actions";
import { useState, useEffect } from "preact/hooks";
import type { Network } from "../../actions/get/networks";
import H2 from "../text/H2";
import Status from "../dynamic/Status";

import { Chart as ChartJS, ArcElement, Tooltip, Legend } from "chart.js";
import { Doughnut } from "react-chartjs-2";

ChartJS.register(ArcElement, Tooltip, Legend);

export default function ({ name }: { name: string }) {
    const [network, setNetwork] = useState<Network | null | undefined>(null);
    useEffect(() => {
        actions.get_network(name).then(nws => {
            setNetwork(nws.error ? undefined : nws.data)
            if (nws.error) console.error(nws.error);
        })
    }, []);

    return (
        <>
            <div class="flex items-center gap-5 h-6">
                <Status status={network?.status} />
                <H2>{name}</H2>
            </div>
            <div className="w-full h-full">
                <Doughnut data={{
                    labels: [
                        'Red',
                        'Blue',
                        'Yellow'
                    ],
                    datasets: [{
                        label: 'My First Dataset',
                        data: [300, 50, 100],
                        backgroundColor: [
                            'rgb(255, 99, 132)',
                            'rgb(54, 162, 235)',
                            'rgb(255, 205, 86)'
                        ],
                        hoverOffset: 4
                    }]
                }} />
            </div>
        </>
    )
}