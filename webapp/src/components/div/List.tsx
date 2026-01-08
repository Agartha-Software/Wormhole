import { toChildArray, type ComponentChild } from "preact";
import type ChildrenProps from "../../ts/ChildrenProps";

function Entry({child}: {child: ComponentChild}) {
    return <li>{child}</li>
}

type ExtendedProps = {
    className?: string;
}
export default function (props: ChildrenProps & ExtendedProps) {
    const children = toChildArray(props.children).map(c => <Entry child={c} />)

    return (
        <ul className={props.className}>
            {children}
        </ul>
    )
}