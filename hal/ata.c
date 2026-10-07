#include "ata.h"
#include "io.h"
#include "serial.h"

static uint8_t current_drive = 0; // 0 = Master (0xA0), 1 = Slave (0xB0)

static void ata_wait_bsy(void) {
    while (inb(ATA_REG_STATUS) & ATA_SR_BSY)
        ;
}

static int ata_wait_drq(void) {
    for (int i = 0; i < 100000; i++) {
        uint8_t s = inb(ATA_REG_STATUS);
        if (s & ATA_SR_ERR)
            return -1;
        if (s & ATA_SR_DRQ)
            return 0;
    }
    return -1;
}

static void ata_400ns_delay(void) {
    inb(ATA_PRIMARY_CTRL);
    inb(ATA_PRIMARY_CTRL);
    inb(ATA_PRIMARY_CTRL);
    inb(ATA_PRIMARY_CTRL);
}

static int ata_identify_drive(uint8_t drive) {
    uint8_t sel = (drive == 1) ? 0xB0 : 0xA0;
    outb(ATA_REG_DRIVE, sel);
    ata_400ns_delay();
    outb(ATA_REG_SECCOUNT, 0);
    outb(ATA_REG_LBA_LO, 0);
    outb(ATA_REG_LBA_MID, 0);
    outb(ATA_REG_LBA_HI, 0);
    outb(ATA_REG_COMMAND, ATA_CMD_IDENTIFY);
    ata_400ns_delay();

    uint8_t status = inb(ATA_REG_STATUS);
    if (status == 0)
        return -1;

    ata_wait_bsy();

    if (inb(ATA_REG_LBA_MID) != 0 || inb(ATA_REG_LBA_HI) != 0)
        return -1;

    if (ata_wait_drq() < 0)
        return -1;

    uint16_t buf[256];
    for (int i = 0; i < 256; i++)
        buf[i] = inw(ATA_REG_DATA);

    uint32_t sectors = (uint32_t)buf[60] | ((uint32_t)buf[61] << 16);
    return (int)sectors;
}

int ata_identify(void) {
    // Check secondary disk (Slave 0xB0) first for persistent storage
    int sectors = ata_identify_drive(1);
    if (sectors > 0) {
        current_drive = 1;
        serial_puts("[ATA] Using dedicated data disk (IDE Slave).\n");
        return sectors;
    }

    // Fall back to primary disk (Master 0xA0)
    sectors = ata_identify_drive(0);
    if (sectors > 0) {
        current_drive = 0;
        serial_puts("[ATA] Using primary disk (IDE Master).\n");
        return sectors;
    }

    return -1;
}

int ata_read_sectors(uint32_t lba, uint8_t count, uint8_t *buf) {
    if (count == 0)
        return -1;

    ata_wait_bsy();

    uint8_t drive_head = (current_drive == 1 ? 0xF0 : 0xE0) | ((lba >> 24) & 0x0F);
    outb(ATA_REG_DRIVE, drive_head);
    outb(ATA_REG_SECCOUNT, count);
    outb(ATA_REG_LBA_LO, (uint8_t)(lba & 0xFF));
    outb(ATA_REG_LBA_MID, (uint8_t)((lba >> 8) & 0xFF));
    outb(ATA_REG_LBA_HI, (uint8_t)((lba >> 16) & 0xFF));
    outb(ATA_REG_COMMAND, ATA_CMD_READ_PIO);

    for (uint8_t sec = 0; sec < count; sec++) {
        ata_400ns_delay();
        if (ata_wait_drq() < 0)
            return -1;

        uint16_t *ptr = (uint16_t *)(buf + sec * 512);
        for (int i = 0; i < 256; i++)
            ptr[i] = inw(ATA_REG_DATA);
    }

    return count;
}

int ata_write_sectors(uint32_t lba, uint8_t count, const uint8_t *buf) {
    if (count == 0)
        return -1;

    ata_wait_bsy();

    uint8_t drive_head = (current_drive == 1 ? 0xF0 : 0xE0) | ((lba >> 24) & 0x0F);
    outb(ATA_REG_DRIVE, drive_head);
    outb(ATA_REG_SECCOUNT, count);
    outb(ATA_REG_LBA_LO, (uint8_t)(lba & 0xFF));
    outb(ATA_REG_LBA_MID, (uint8_t)((lba >> 8) & 0xFF));
    outb(ATA_REG_LBA_HI, (uint8_t)((lba >> 16) & 0xFF));
    outb(ATA_REG_COMMAND, ATA_CMD_WRITE_PIO);

    for (uint8_t sec = 0; sec < count; sec++) {
        ata_400ns_delay();
        if (ata_wait_drq() < 0)
            return -1;

        const uint16_t *ptr = (const uint16_t *)(buf + sec * 512);
        for (int i = 0; i < 256; i++)
            outw(ATA_REG_DATA, ptr[i]);

        outb(ATA_REG_COMMAND, ATA_CMD_FLUSH);
        ata_wait_bsy();
    }

    return count;
}
