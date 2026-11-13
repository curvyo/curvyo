import { useCallback, useEffect, useRef, useState } from "react";

/** The camera of the photo controls (`specs/0112` `ux-notes.md` 8.5): offered only with a camera
 * API and a permission that is not refused; the stream stops on `stop`, on unmount (a tab or test
 * change) and when the view ends. */
export interface CameraApi {
  /** The button shows: the API exists and the permission is not refused. */
  offered: boolean;
  /** The live view is open. */
  active: boolean;
  /** The callback ref of the `<video>` that shows the stream. */
  attach: (element: HTMLVideoElement | null) => void;
  /** The video element now, for the frame to take. */
  video: () => HTMLVideoElement | null;
  /** Opens the live view; the refusal text ("Camera access was refused.") when it could not. */
  start: () => Promise<string | null>;
  stop: () => void;
}

const REFUSED = "Camera access was refused.";

/** The camera for one photo section. */
export function useCamera(): CameraApi {
  const supported = typeof navigator !== "undefined" && typeof navigator.mediaDevices?.getUserMedia === "function";
  const [denied, setDenied] = useState(false);
  const [active, setActive] = useState(false);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);

  const stop = useCallback(() => {
    streamRef.current?.getTracks().forEach((track) => track.stop());
    streamRef.current = null;
    if (videoRef.current) videoRef.current.srcObject = null;
    setActive(false);
  }, []);

  useEffect(() => {
    let alive = true;
    if (supported && navigator.permissions?.query) {
      navigator.permissions
        .query({ name: "camera" as PermissionName })
        .then((status) => {
          if (alive && status.state === "denied") setDenied(true);
        })
        .catch(() => {});
    }
    return () => {
      alive = false;
    };
  }, [supported]);

  // The stream stops with the section: a tab change or another test unmounts it.
  useEffect(() => stop, [stop]);

  const start = useCallback(async (): Promise<string | null> => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ video: true });
      streamRef.current = stream;
      setActive(true);
      // The video element mounts with `active`; the stream is attached once it is there.
      window.requestAnimationFrame(() => {
        const video = videoRef.current;
        if (video) {
          video.srcObject = stream;
          void video.play().catch(() => {});
        }
      });
      return null;
    } catch {
      setDenied(true);
      return REFUSED;
    }
  }, []);

  // The video element can mount after the stream is open (the live view shows once `start`
  // answered), so it takes the open stream when it mounts.
  const attach = useCallback((element: HTMLVideoElement | null) => {
    videoRef.current = element;
    const stream = streamRef.current;
    if (element && stream && element.srcObject !== stream) {
      element.srcObject = stream;
      void element.play().catch(() => {});
    }
  }, []);
  const video = useCallback(() => videoRef.current, []);
  return { offered: supported && !denied, active, attach, video, start, stop };
}
