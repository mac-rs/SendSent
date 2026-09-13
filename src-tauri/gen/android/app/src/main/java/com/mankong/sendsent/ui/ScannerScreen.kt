package com.mankong.sendsent.ui

import android.annotation.SuppressLint
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.google.zxing.BinaryBitmap
import com.google.zxing.MultiFormatReader
import com.google.zxing.NotFoundException
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.common.HybridBinarizer
import java.util.concurrent.Executors

/** CameraX + ZXing 扫码；解出首个二维码后回调一次。 */
@Composable
fun ScannerScreen(onDecoded: (String) -> Unit) {
    val ctx = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    var handled by remember { mutableStateOf(false) }
    val executor = remember { Executors.newSingleThreadExecutor() }
    val reader = remember { MultiFormatReader() }

    DisposableEffect(Unit) { onDispose { executor.shutdown() } }

    Box(Modifier.fillMaxSize()) {
        AndroidView(
            factory = { c ->
                val preview = PreviewView(c)
                val providerFuture = ProcessCameraProvider.getInstance(c)
                providerFuture.addListener({
                    val provider = providerFuture.get()
                    val previewUse = Preview.Builder().build().also {
                        it.setSurfaceProvider(preview.surfaceProvider)
                    }
                    val analysis = ImageAnalysis.Builder()
                        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                        .build()
                    analysis.setAnalyzer(executor) { image ->
                        if (handled) { image.close(); return@setAnalyzer }
                        val source = image.toLuminance()
                        if (source != null) {
                            try {
                                val result = reader.decodeWithState(BinaryBitmap(HybridBinarizer(source)))
                                handled = true
                                val text = result.text
                                ContextCompat.getMainExecutor(c).execute { onDecoded(text) }
                            } catch (_: NotFoundException) {
                            } catch (_: Exception) {
                            } finally {
                                reader.reset()
                            }
                        }
                        image.close()
                    }
                    runCatching {
                        provider.unbindAll()
                        provider.bindToLifecycle(
                            lifecycleOwner, CameraSelector.DEFAULT_BACK_CAMERA, previewUse, analysis,
                        )
                    }
                }, ContextCompat.getMainExecutor(c))
                preview
            },
            modifier = Modifier.fillMaxSize(),
        )
    }
}

@SuppressLint("UnsafeOptInUsageError")
private fun ImageProxy.toLuminance(): PlanarYUVLuminanceSource? {
    if (planes.isEmpty()) return null
    val buffer = planes[0].buffer
    val data = ByteArray(buffer.remaining())
    buffer.get(data)
    return PlanarYUVLuminanceSource(data, width, height, 0, 0, width, height, false)
}
