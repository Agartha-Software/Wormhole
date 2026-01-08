import type ChildrenProps from "../../ts/ChildrenProps";

export default function (props: ChildrenProps) {
    return (
        <h2 class="text-3xl font-extrabold">{props.children}</h2>
    )
}