import { MediaPage } from "./Music";
import type { PageProps } from "../App";

export default function Video(props: PageProps) {
  return <MediaPage kind="video" {...props} />;
}
