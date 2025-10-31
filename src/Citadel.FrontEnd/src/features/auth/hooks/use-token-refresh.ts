import { useCallback, useEffect, useRef } from "react";
import { jwtDecode } from "jwt-decode";
import { useInterval } from "@/hooks/useInterval";
import { useRead } from "@/lib/hooks";

const EXPIRY_BUFFER_INTERVAL = 30; // seconds before expiry to refresh
const EXPIRY_BUFFER_ON_FOCUS = 10; // when tab becomes visible

export const useTokenRefresh = (
  accessToken: string | undefined,
  isAuthenticated: boolean
) => {
  const { data, isSuccess, refetch, error } = useRead("refreshToken", undefined, {
    enabled: false,
  });

  const isRefreshing = useRef(false);
  const didAttemptRefresh = useRef(false);

  const parseJwt = (token: string) => {
    try {
      return token ? (jwtDecode(token) as { exp?: number }) : null;
    } catch {
      return null;
    }
  };

  const triggerRefresh = useCallback(() => {
    if (isRefreshing.current) return;

    isRefreshing.current = true;
    didAttemptRefresh.current = true;

    refetch().finally(() => {
      isRefreshing.current = false;
    });
  }, [refetch]);

  const maybeRefreshIfExpiringSoon = useCallback(
    (bufferSeconds: number) => {
      if (!isAuthenticated || !accessToken) return;
      const decoded = parseJwt(accessToken);
      if (!decoded?.exp) return;

      const secondsLeft = decoded.exp - Math.floor(Date.now() / 1000);
      if (secondsLeft < bufferSeconds) triggerRefresh();
    },
    [accessToken, isAuthenticated, triggerRefresh]
  );

  // Periodic refresh check
  useInterval(() => maybeRefreshIfExpiringSoon(EXPIRY_BUFFER_INTERVAL),
              isAuthenticated ? 10000 : null);

  // Refresh when user focuses the tab
  useEffect(() => {
    const onVisible = () => {
      if (document.visibilityState === "visible") {
        maybeRefreshIfExpiringSoon(EXPIRY_BUFFER_ON_FOCUS);
      }
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [maybeRefreshIfExpiringSoon]);

  return {
    token: data?.data?.accessToken,
    isSuccess,
    error,
    didAttemptRefresh,
  };
};
