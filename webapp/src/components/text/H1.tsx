import type ChildrenProps from "../../ts/ChildrenProps";

export default function (props: ChildrenProps) {
    return (
        <h1 class="text-6xl font-black">{props.children}</h1>
    )
}