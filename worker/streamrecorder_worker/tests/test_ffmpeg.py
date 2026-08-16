from pathlib import Path
import unittest

from streamrecorder_worker.ffmpeg import build_ffmpeg_command


class BuildFfmpegCommandTests(unittest.TestCase):
    @staticmethod
    def _build_command(record_url: str) -> list[str]:
        return build_ffmpeg_command(
            record_url,
            Path("recording.ts"),
            "TS",
            segment_record=False,
            segment_time="1800",
        )

    def test_http_inputs_enable_reconnect_before_input(self) -> None:
        for record_url in (
            "http://example.test/live.flv",
            "HTTPS://example.test/live.m3u8",
        ):
            with self.subTest(record_url=record_url):
                command = self._build_command(record_url)
                input_index = command.index("-i")

                self.assertEqual(
                    command[input_index - 8 : input_index + 2],
                    [
                        "-reconnect",
                        "1",
                        "-reconnect_at_eof",
                        "1",
                        "-reconnect_streamed",
                        "1",
                        "-reconnect_delay_max",
                        "15",
                        "-i",
                        record_url,
                    ],
                )
                self.assertNotIn("-reconnect_on_network_error", command)

    def test_non_http_input_does_not_enable_http_reconnect(self) -> None:
        command = self._build_command("rtmp://example.test/live")

        self.assertNotIn("-reconnect", command)


if __name__ == "__main__":
    unittest.main()
