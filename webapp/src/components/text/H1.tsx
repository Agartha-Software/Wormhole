import type ChildrenProps from "../../ts/ChildrenProps";

export default function (props: ChildrenProps) {
    return (
        <h1 class="text-5xl font-black">{props.children}</h1>
    )
}