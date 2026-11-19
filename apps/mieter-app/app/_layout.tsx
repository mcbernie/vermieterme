import { useEffect, useState } from "react";
import { Stack, useRouter, useSegments } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { View, ActivityIndicator } from "react-native";
import { SafeAreaProvider } from "react-native-safe-area-context";
import { isAuthenticated } from "../lib/auth";

export default function RootLayout() {
  const [checking, setChecking] = useState(true);
  const [authed, setAuthed] = useState(false);
  const router = useRouter();
  const segments = useSegments();

  useEffect(() => {
    isAuthenticated().then((result) => {
      setAuthed(result);
      setChecking(false);
    });
  }, []);

  useEffect(() => {
    if (checking) return;

    const inAuthGroup = segments[0] === "login";

    if (!authed && !inAuthGroup) {
      router.replace("/login");
    } else if (authed && inAuthGroup) {
      router.replace("/");
    }
  }, [checking, authed, segments, router]);

  if (checking) {
    return (
      <SafeAreaProvider>
        <View style={{ flex: 1, justifyContent: "center", alignItems: "center", backgroundColor: "#fff" }}>
          <ActivityIndicator size="large" color="#b91c1c" />
          <StatusBar style="dark" />
        </View>
      </SafeAreaProvider>
    );
  }

  return (
    <SafeAreaProvider>
      <StatusBar style="dark" />
      <Stack
        screenOptions={{
          headerTintColor: "#b91c1c",
          headerBackTitle: "Zurück",
        }}
      >
        <Stack.Screen name="login" options={{ headerShown: false }} />
        <Stack.Screen name="(tabs)" options={{ headerShown: false }} />
        <Stack.Screen
          name="billing/[id]"
          options={{ title: "Abrechnung" }}
        />
      </Stack>
    </SafeAreaProvider>
  );
}
